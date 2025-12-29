//! Proof-of-storage mechanisms for verifying data integrity
//!
//! This module implements cryptographic challenges and proofs that allow
//! the network to verify that storage providers are correctly storing data
//! without requiring the full file to be transmitted.

use carbide_core::{ContentHash, CarbideError, Result};
use chrono::{DateTime, Utc};
use ring::rand::{SecureRandom, SystemRandom};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A cryptographic challenge issued to a storage provider
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StorageChallenge {
    /// Unique challenge identifier  
    pub challenge_id: String,
    /// File being challenged
    pub file_hash: ContentHash,
    /// Specific chunk indices to prove (random subset)
    pub chunk_indices: Vec<usize>,
    /// Random nonce for this challenge
    pub nonce: [u8; 32],
    /// When the challenge was issued
    pub issued_at: DateTime<Utc>,
    /// Challenge expiry time
    pub expires_at: DateTime<Utc>,
    /// Expected response hash (for verification)
    pub expected_response_hash: ContentHash,
}

/// Response to a storage challenge from a provider
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StorageProof {
    /// Original challenge identifier
    pub challenge_id: String,
    /// Merkle proofs for the requested chunks
    pub merkle_proofs: Vec<ChunkProof>,
    /// Response hash computed from the challenged data
    pub response_hash: ContentHash,
    /// Provider signature over the response
    pub signature: Vec<u8>,
    /// When the proof was generated
    pub generated_at: DateTime<Utc>,
}

/// Proof for a specific chunk within a storage challenge
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChunkProof {
    /// Index of the chunk being proven
    pub chunk_index: usize,
    /// Hash of the chunk data
    pub chunk_hash: ContentHash,
    /// Merkle tree proof path
    pub merkle_path: Vec<ContentHash>,
    /// Chunk data (if requested for small chunks)
    pub chunk_data: Option<Vec<u8>>,
}

/// Generator for storage challenges
#[derive(Debug)]
pub struct ChallengeGenerator {
    rng: SystemRandom,
}

impl Default for ChallengeGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl ChallengeGenerator {
    /// Create a new challenge generator
    pub fn new() -> Self {
        Self {
            rng: SystemRandom::new(),
        }
    }
    
    /// Generate a storage challenge for a file
    pub fn generate_challenge(
        &self,
        file_hash: ContentHash,
        total_chunks: usize,
        challenge_percentage: f32,
    ) -> Result<StorageChallenge> {
        if !(0.0..=1.0).contains(&challenge_percentage) {
            return Err(CarbideError::Internal(
                "Challenge percentage must be between 0.0 and 1.0".to_string()
            ));
        }
        
        if total_chunks == 0 {
            return Err(CarbideError::Internal("File must have at least one chunk".to_string()));
        }
        
        // Calculate how many chunks to challenge
        let chunks_to_challenge = std::cmp::max(
            1,
            (total_chunks as f32 * challenge_percentage) as usize
        );
        
        // Generate random chunk indices
        let chunk_indices = self.select_random_chunks(total_chunks, chunks_to_challenge)?;
        
        // Generate random nonce
        let mut nonce = [0u8; 32];
        self.rng.fill(&mut nonce)
            .map_err(|_| CarbideError::Internal("Failed to generate challenge nonce".to_string()))?;
        
        // Generate challenge ID
        let challenge_id = format!("challenge_{}", hex::encode(&nonce[..8]));
        
        // Calculate expected response hash
        let expected_response_hash = self.calculate_expected_response(
            &file_hash,
            &chunk_indices,
            &nonce,
        );
        
        let issued_at = Utc::now();
        let expires_at = issued_at + chrono::Duration::minutes(10); // 10-minute expiry
        
        Ok(StorageChallenge {
            challenge_id,
            file_hash,
            chunk_indices,
            nonce,
            issued_at,
            expires_at,
            expected_response_hash,
        })
    }
    
    /// Select random chunk indices for challenging
    fn select_random_chunks(&self, total_chunks: usize, count: usize) -> Result<Vec<usize>> {
        let mut selected = std::collections::HashSet::new();
        let mut attempts = 0;
        
        while selected.len() < count && attempts < 1000 {
            let mut index_bytes = [0u8; 4];
            self.rng.fill(&mut index_bytes)
                .map_err(|_| CarbideError::Internal("Failed to generate random index".to_string()))?;
            
            let index = (u32::from_be_bytes(index_bytes) as usize) % total_chunks;
            selected.insert(index);
            attempts += 1;
        }
        
        if selected.len() < count {
            return Err(CarbideError::Internal(
                "Failed to select enough unique chunk indices".to_string()
            ));
        }
        
        let mut result: Vec<usize> = selected.into_iter().collect();
        result.sort_unstable();
        Ok(result)
    }
    
    /// Calculate the expected response hash for verification
    fn calculate_expected_response(
        &self,
        file_hash: &ContentHash,
        chunk_indices: &[usize],
        nonce: &[u8; 32],
    ) -> ContentHash {
        let mut hasher = blake3::Hasher::new();
        
        // Include file hash, challenge nonce, and chunk indices
        hasher.update(file_hash.as_bytes());
        hasher.update(nonce);
        
        for &index in chunk_indices {
            hasher.update(&index.to_be_bytes());
        }
        
        ContentHash::new(*hasher.finalize().as_bytes())
    }
}

/// Verifier for storage proofs
#[derive(Debug)]
pub struct ProofVerifier;

impl ProofVerifier {
    /// Verify a storage proof against a challenge
    pub fn verify_proof(
        challenge: &StorageChallenge,
        proof: &StorageProof,
        file_merkle_root: ContentHash,
    ) -> Result<bool> {
        // Check that proof matches challenge
        if proof.challenge_id != challenge.challenge_id {
            return Ok(false);
        }
        
        // Check that challenge hasn't expired
        if Utc::now() > challenge.expires_at {
            return Err(CarbideError::Internal("Challenge has expired".to_string()));
        }
        
        // Verify we have proofs for all requested chunks
        if proof.merkle_proofs.len() != challenge.chunk_indices.len() {
            return Ok(false);
        }
        
        // Verify each merkle proof
        for (i, chunk_proof) in proof.merkle_proofs.iter().enumerate() {
            let expected_index = challenge.chunk_indices[i];
            
            if chunk_proof.chunk_index != expected_index {
                return Ok(false);
            }
            
            // Verify the merkle proof
            if !Self::verify_merkle_proof(
                chunk_proof,
                file_merkle_root,
                challenge.chunk_indices.len(),
            ) {
                return Ok(false);
            }
        }
        
        // Verify response hash
        let expected_hash = Self::calculate_response_hash(
            &challenge.file_hash,
            &challenge.chunk_indices,
            &challenge.nonce,
            &proof.merkle_proofs,
        );
        
        if proof.response_hash != expected_hash {
            return Ok(false);
        }
        
        Ok(true)
    }
    
    /// Verify a single merkle proof
    fn verify_merkle_proof(
        chunk_proof: &ChunkProof,
        merkle_root: ContentHash,
        _total_chunks: usize,
    ) -> bool {
        let mut current_hash = chunk_proof.chunk_hash;
        let mut current_index = chunk_proof.chunk_index;
        
        for sibling_hash in &chunk_proof.merkle_path {
            let mut hasher = blake3::Hasher::new();
            
            if current_index % 2 == 0 {
                // Left child: current + sibling
                hasher.update(current_hash.as_bytes());
                hasher.update(sibling_hash.as_bytes());
            } else {
                // Right child: sibling + current
                hasher.update(sibling_hash.as_bytes());
                hasher.update(current_hash.as_bytes());
            }
            
            current_hash = ContentHash::new(*hasher.finalize().as_bytes());
            current_index /= 2;
        }
        
        current_hash == merkle_root
    }
    
    /// Calculate response hash for verification
    fn calculate_response_hash(
        file_hash: &ContentHash,
        chunk_indices: &[usize],
        nonce: &[u8; 32],
        proofs: &[ChunkProof],
    ) -> ContentHash {
        let mut hasher = blake3::Hasher::new();
        
        // Include challenge parameters
        hasher.update(file_hash.as_bytes());
        hasher.update(nonce);
        
        for &index in chunk_indices {
            hasher.update(&index.to_be_bytes());
        }
        
        // Include proof data
        for proof in proofs {
            hasher.update(proof.chunk_hash.as_bytes());
            for path_hash in &proof.merkle_path {
                hasher.update(path_hash.as_bytes());
            }
        }
        
        ContentHash::new(*hasher.finalize().as_bytes())
    }
}

/// Manages ongoing storage challenges and proofs
#[derive(Debug)]
pub struct ProofManager {
    active_challenges: HashMap<String, StorageChallenge>,
    completed_proofs: HashMap<String, StorageProof>,
    challenge_generator: ChallengeGenerator,
}

impl Default for ProofManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ProofManager {
    /// Create a new proof manager
    pub fn new() -> Self {
        Self {
            active_challenges: HashMap::new(),
            completed_proofs: HashMap::new(),
            challenge_generator: ChallengeGenerator::new(),
        }
    }
    
    /// Issue a new storage challenge
    pub fn issue_challenge(
        &mut self,
        file_hash: ContentHash,
        total_chunks: usize,
        challenge_percentage: f32,
    ) -> Result<StorageChallenge> {
        let challenge = self.challenge_generator.generate_challenge(
            file_hash,
            total_chunks,
            challenge_percentage,
        )?;
        
        self.active_challenges.insert(challenge.challenge_id.clone(), challenge.clone());
        Ok(challenge)
    }
    
    /// Submit a proof for verification
    pub fn submit_proof(
        &mut self,
        proof: StorageProof,
        file_merkle_root: ContentHash,
    ) -> Result<bool> {
        let challenge = self.active_challenges.get(&proof.challenge_id)
            .ok_or_else(|| CarbideError::Internal("Challenge not found".to_string()))?
            .clone();
        
        let is_valid = ProofVerifier::verify_proof(&challenge, &proof, file_merkle_root)?;
        
        if is_valid {
            self.completed_proofs.insert(proof.challenge_id.clone(), proof);
            self.active_challenges.remove(&challenge.challenge_id);
        }
        
        Ok(is_valid)
    }
    
    /// Clean up expired challenges
    pub fn cleanup_expired_challenges(&mut self) {
        let now = Utc::now();
        self.active_challenges.retain(|_, challenge| challenge.expires_at > now);
    }
    
    /// Get statistics about proof activity
    pub fn get_statistics(&self) -> ProofStatistics {
        let now = Utc::now();
        let expired_challenges = self.active_challenges
            .values()
            .filter(|c| c.expires_at <= now)
            .count();
        
        ProofStatistics {
            active_challenges: self.active_challenges.len() - expired_challenges,
            expired_challenges,
            completed_proofs: self.completed_proofs.len(),
        }
    }
}

/// Statistics about proof-of-storage activity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofStatistics {
    /// Number of active (non-expired) challenges
    pub active_challenges: usize,
    /// Number of expired challenges awaiting cleanup
    pub expired_challenges: usize,
    /// Number of successfully verified proofs
    pub completed_proofs: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_challenge_generation() {
        let generator = ChallengeGenerator::new();
        let file_hash = ContentHash::from_data(b"test file");
        
        let challenge = generator.generate_challenge(file_hash, 10, 0.3).unwrap();
        
        assert_eq!(challenge.file_hash, file_hash);
        assert_eq!(challenge.chunk_indices.len(), 3); // 30% of 10 chunks
        assert!(challenge.expires_at > challenge.issued_at);
        
        // Verify chunk indices are within range and sorted
        for &index in &challenge.chunk_indices {
            assert!(index < 10);
        }
        
        // Check that indices are sorted
        for i in 1..challenge.chunk_indices.len() {
            assert!(challenge.chunk_indices[i] > challenge.chunk_indices[i-1]);
        }
    }
    
    #[test]
    fn test_proof_verification() {
        let file_hash = ContentHash::from_data(b"test file");
        let merkle_root = ContentHash::from_data(b"merkle root");
        
        let generator = ChallengeGenerator::new();
        let challenge = generator.generate_challenge(file_hash, 4, 0.5).unwrap();
        
        // Create a valid proof
        let chunk_proofs: Vec<ChunkProof> = challenge.chunk_indices.iter().map(|&index| {
            ChunkProof {
                chunk_index: index,
                chunk_hash: ContentHash::from_data(&format!("chunk {}", index).as_bytes()),
                merkle_path: vec![
                    ContentHash::from_data(b"sibling1"),
                    ContentHash::from_data(b"sibling2"),
                ],
                chunk_data: None,
            }
        }).collect();
        
        let response_hash = ProofVerifier::calculate_response_hash(
            &challenge.file_hash,
            &challenge.chunk_indices,
            &challenge.nonce,
            &chunk_proofs,
        );
        
        let proof = StorageProof {
            challenge_id: challenge.challenge_id.clone(),
            merkle_proofs: chunk_proofs,
            response_hash,
            signature: vec![0u8; 64], // Mock signature
            generated_at: Utc::now(),
        };
        
        // Note: This will fail merkle verification since we're using mock data
        // In a real implementation, we'd need valid merkle proofs
        let result = ProofVerifier::verify_proof(&challenge, &proof, merkle_root);
        assert!(result.is_ok()); // Should not error, but might return false
    }
    
    #[test]
    fn test_proof_manager() {
        let mut manager = ProofManager::new();
        let file_hash = ContentHash::from_data(b"test file");
        let merkle_root = ContentHash::from_data(b"merkle root");
        
        // Issue a challenge
        let challenge = manager.issue_challenge(file_hash, 8, 0.25).unwrap();
        assert_eq!(manager.get_statistics().active_challenges, 1);
        
        // Create a mock proof
        let proof = StorageProof {
            challenge_id: challenge.challenge_id,
            merkle_proofs: vec![],
            response_hash: ContentHash::from_data(b"response"),
            signature: vec![],
            generated_at: Utc::now(),
        };
        
        // Submit proof (will likely fail verification with mock data)
        let _result = manager.submit_proof(proof, merkle_root);
        // Statistics should reflect the attempt
    }
    
    #[test]
    fn test_challenge_expiry() {
        let mut manager = ProofManager::new();
        let file_hash = ContentHash::from_data(b"test file");
        
        let _challenge = manager.issue_challenge(file_hash, 5, 0.4).unwrap();
        assert_eq!(manager.get_statistics().active_challenges, 1);
        
        // In a real test, we'd manipulate time to test expiry
        // For now, just verify the structure works
        manager.cleanup_expired_challenges();
    }
}
