//! Content-addressing and file chunking for efficient storage
//!
//! This module implements BLAKE3-based content addressing similar to IPFS,
//! with efficient chunking for large files like Storj's 64MB chunks.

use std::io::{Read, Seek, SeekFrom};

use blake3::Hasher;
use carbide_core::{CarbideError, ContentHash, FileChunk, Result};
use serde::{Deserialize, Serialize};

/// Maximum chunk size (64MB like Storj)
pub const MAX_CHUNK_SIZE: u64 = 64 * 1024 * 1024;

/// Minimum chunk size for small files (1MB)
pub const MIN_CHUNK_SIZE: u64 = 1024 * 1024;

/// File chunker for splitting large files into manageable pieces
#[derive(Debug, Clone)]
pub struct FileChunker {
    /// Target chunk size (default 64MB)
    pub chunk_size: u64,
}

impl Default for FileChunker {
    fn default() -> Self {
        Self {
            chunk_size: MAX_CHUNK_SIZE,
        }
    }
}

impl FileChunker {
    /// Create a new chunker with custom chunk size
    pub fn new(chunk_size: u64) -> Result<Self> {
        if !(MIN_CHUNK_SIZE..=MAX_CHUNK_SIZE).contains(&chunk_size) {
            return Err(CarbideError::Internal(format!(
                "Chunk size must be between {}MB and {}MB",
                MIN_CHUNK_SIZE / (1024 * 1024),
                MAX_CHUNK_SIZE / (1024 * 1024)
            )));
        }

        Ok(Self { chunk_size })
    }

    /// Split file data into chunks
    pub fn chunk_data(&self, data: &[u8]) -> Vec<FileChunk> {
        if data.len() as u64 <= self.chunk_size {
            // File is small enough to be a single chunk
            return vec![FileChunk {
                hash: ContentHash::from_data(data),
                data: data.to_vec(),
                offset: 0,
                total_size: data.len() as u64,
            }];
        }

        let mut chunks = Vec::new();
        let mut offset = 0;
        let total_size = data.len() as u64;

        while offset < data.len() {
            let chunk_end = std::cmp::min(offset + self.chunk_size as usize, data.len());
            let chunk_data = &data[offset..chunk_end];

            chunks.push(FileChunk {
                hash: ContentHash::from_data(chunk_data),
                data: chunk_data.to_vec(),
                offset: offset as u64,
                total_size,
            });

            offset = chunk_end;
        }

        chunks
    }

    /// Split a file reader into chunks (for streaming large files)
    pub fn chunk_reader<R: Read + Seek>(&self, reader: &mut R) -> Result<Vec<FileChunk>> {
        // Get total file size
        let current_pos = reader
            .stream_position()
            .map_err(|e| CarbideError::Internal(format!("Failed to get stream position: {e}")))?;

        let total_size = reader
            .seek(SeekFrom::End(0))
            .map_err(|e| CarbideError::Internal(format!("Failed to seek to end: {e}")))?;

        reader
            .seek(SeekFrom::Start(current_pos))
            .map_err(|e| CarbideError::Internal(format!("Failed to restore position: {e}")))?;

        let mut chunks = Vec::new();
        let mut buffer = vec![0u8; self.chunk_size as usize];
        let mut offset = 0;

        loop {
            let bytes_read = reader
                .read(&mut buffer)
                .map_err(|e| CarbideError::Internal(format!("Failed to read chunk: {e}")))?;

            if bytes_read == 0 {
                break;
            }

            let chunk_data = &buffer[..bytes_read];
            chunks.push(FileChunk {
                hash: ContentHash::from_data(chunk_data),
                data: chunk_data.to_vec(),
                offset,
                total_size,
            });

            offset += bytes_read as u64;
        }

        Ok(chunks)
    }

    /// Reassemble chunks back into original data
    pub fn reassemble_chunks(chunks: &[FileChunk]) -> Result<Vec<u8>> {
        if chunks.is_empty() {
            return Ok(Vec::new());
        }

        // Sort chunks by offset to ensure correct order
        let mut sorted_chunks = chunks.to_vec();
        sorted_chunks.sort_by_key(|chunk| chunk.offset);

        // Verify chunks form a complete file
        let total_size = sorted_chunks[0].total_size;
        let mut expected_offset = 0;

        for chunk in &sorted_chunks {
            if chunk.offset != expected_offset {
                return Err(CarbideError::Internal(format!(
                    "Missing chunk at offset {expected_offset}"
                )));
            }

            if chunk.total_size != total_size {
                return Err(CarbideError::Internal(
                    "Chunks have inconsistent total size".to_string(),
                ));
            }

            expected_offset += chunk.data.len() as u64;
        }

        if expected_offset != total_size {
            return Err(CarbideError::Internal(format!(
                "Chunks total {expected_offset} bytes but expected {total_size}"
            )));
        }

        // Reassemble data
        let mut data = Vec::with_capacity(total_size as usize);
        for chunk in sorted_chunks {
            data.extend_from_slice(&chunk.data);
        }

        Ok(data)
    }
}

/// Content-addressed file manager
#[derive(Debug, Clone, Default)]
pub struct ContentStore {
    chunker: FileChunker,
}

impl ContentStore {
    /// Create a new content store
    pub fn new(chunker: FileChunker) -> Self {
        Self { chunker }
    }

    /// Process file data into content-addressed chunks
    pub fn process_file(&self, data: &[u8]) -> ProcessedFile {
        let chunks = self.chunker.chunk_data(data);
        let file_hash = ContentHash::from_data(data);

        ProcessedFile {
            file_hash,
            chunks,
            total_size: data.len() as u64,
        }
    }

    /// Verify chunk integrity
    pub fn verify_chunk(&self, chunk: &FileChunk) -> bool {
        let expected_hash = ContentHash::from_data(&chunk.data);
        chunk.hash == expected_hash
    }

    /// Verify all chunks and reassemble if valid
    pub fn verify_and_reassemble(&self, chunks: &[FileChunk]) -> Result<Vec<u8>> {
        // Verify each chunk's integrity
        for chunk in chunks {
            if !self.verify_chunk(chunk) {
                return Err(CarbideError::Internal(format!(
                    "Chunk at offset {} failed integrity check",
                    chunk.offset
                )));
            }
        }

        // Reassemble if all chunks are valid
        FileChunker::reassemble_chunks(chunks)
    }
}

/// Result of processing a file through content addressing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedFile {
    /// Hash of the complete file
    pub file_hash: ContentHash,
    /// File chunks with individual hashes
    pub chunks: Vec<FileChunk>,
    /// Total file size
    pub total_size: u64,
}

impl ProcessedFile {
    /// Get all chunk hashes
    pub fn chunk_hashes(&self) -> Vec<ContentHash> {
        self.chunks.iter().map(|chunk| chunk.hash).collect()
    }

    /// Calculate how many chunks this file has
    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    /// Check if this is a single-chunk file
    pub fn is_single_chunk(&self) -> bool {
        self.chunks.len() == 1
    }
}

/// Merkle tree builder for efficient integrity verification
#[derive(Debug)]
pub struct MerkleTreeBuilder;

impl MerkleTreeBuilder {
    /// Build a Merkle tree from chunk hashes
    pub fn build_tree(chunk_hashes: &[ContentHash]) -> Result<MerkleTree> {
        if chunk_hashes.is_empty() {
            return Err(CarbideError::Internal(
                "Cannot build tree from empty chunks".to_string(),
            ));
        }

        // Start with leaf nodes (chunk hashes)
        let mut current_level: Vec<ContentHash> = chunk_hashes.to_vec();
        let mut tree = MerkleTree {
            root: ContentHash::from_data(&[]),
            levels: vec![current_level.clone()],
        };

        // Build tree bottom-up
        while current_level.len() > 1 {
            let mut next_level = Vec::new();

            for pair in current_level.chunks(2) {
                let combined_hash = if pair.len() == 2 {
                    // Hash the concatenation of two hashes
                    let mut hasher = Hasher::new();
                    hasher.update(pair[0].as_bytes());
                    hasher.update(pair[1].as_bytes());
                    ContentHash::new(*hasher.finalize().as_bytes())
                } else {
                    // Odd number of nodes, promote the last one
                    pair[0]
                };

                next_level.push(combined_hash);
            }

            tree.levels.push(next_level.clone());
            current_level = next_level;
        }

        tree.root = current_level[0];
        Ok(tree)
    }
}

/// Merkle tree for efficient file integrity verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleTree {
    /// Root hash of the tree
    pub root: ContentHash,
    /// All levels of the tree (leaves at index 0)
    pub levels: Vec<Vec<ContentHash>>,
}

impl MerkleTree {
    /// Get the root hash
    pub fn root_hash(&self) -> ContentHash {
        self.root
    }

    /// Get proof path for a specific chunk index
    pub fn get_proof(&self, chunk_index: usize) -> Result<MerkleProof> {
        if self.levels.is_empty() {
            return Err(CarbideError::Internal("Empty Merkle tree".to_string()));
        }

        let leaf_count = self.levels[0].len();
        if chunk_index >= leaf_count {
            return Err(CarbideError::Internal(format!(
                "Chunk index {} out of bounds (max {})",
                chunk_index,
                leaf_count - 1
            )));
        }

        let mut proof_hashes = Vec::new();
        let mut current_index = chunk_index;

        // Traverse up the tree, collecting sibling hashes
        for level in &self.levels[..self.levels.len() - 1] {
            let sibling_index = if current_index.is_multiple_of(2) {
                current_index + 1
            } else {
                current_index - 1
            };

            if sibling_index < level.len() {
                proof_hashes.push(level[sibling_index]);
            }

            current_index /= 2;
        }

        Ok(MerkleProof {
            chunk_index,
            chunk_hash: self.levels[0][chunk_index],
            proof_hashes,
            root_hash: self.root,
        })
    }

    /// Verify a Merkle proof
    pub fn verify_proof(&self, proof: &MerkleProof) -> bool {
        proof.verify()
    }
}

/// Merkle proof for a specific chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleProof {
    /// Index of the chunk being proven
    pub chunk_index: usize,
    /// Hash of the chunk
    pub chunk_hash: ContentHash,
    /// Sibling hashes along the path to root
    pub proof_hashes: Vec<ContentHash>,
    /// Expected root hash
    pub root_hash: ContentHash,
}

impl MerkleProof {
    /// Verify this proof is valid
    pub fn verify(&self) -> bool {
        let mut current_hash = self.chunk_hash;
        let mut current_index = self.chunk_index;

        for sibling_hash in &self.proof_hashes {
            let mut hasher = Hasher::new();

            // Order hashes based on position
            if current_index.is_multiple_of(2) {
                // Left child: current_hash + sibling_hash
                hasher.update(current_hash.as_bytes());
                hasher.update(sibling_hash.as_bytes());
            } else {
                // Right child: sibling_hash + current_hash
                hasher.update(sibling_hash.as_bytes());
                hasher.update(current_hash.as_bytes());
            }

            current_hash = ContentHash::new(*hasher.finalize().as_bytes());
            current_index /= 2;
        }

        current_hash == self.root_hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_chunking() {
        let chunker = FileChunker::new(MIN_CHUNK_SIZE).unwrap(); // 1MB chunks for testing

        // Test small file (no chunking needed)
        let small_data = b"Hello, Carbide Network!";
        let chunks = chunker.chunk_data(small_data);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].data, small_data);
        assert_eq!(chunks[0].offset, 0);

        // Test large file (needs chunking)
        let large_data = vec![42u8; 2 * 1024 * 1024 + 500 * 1024]; // 2.5MB data
        let chunks = chunker.chunk_data(&large_data);
        assert_eq!(chunks.len(), 3); // Should split into 3 chunks

        // Verify chunk properties
        assert_eq!(chunks[0].offset, 0);
        assert_eq!(chunks[1].offset, MIN_CHUNK_SIZE);
        assert_eq!(chunks[2].offset, 2 * MIN_CHUNK_SIZE);
        assert_eq!(chunks[0].data.len(), MIN_CHUNK_SIZE as usize);
        assert_eq!(chunks[1].data.len(), MIN_CHUNK_SIZE as usize);
        assert_eq!(chunks[2].data.len(), 500 * 1024); // 500KB remainder
    }

    #[test]
    fn test_chunk_reassembly() {
        let chunker = FileChunker::new(MIN_CHUNK_SIZE).unwrap();
        let original_data = vec![42u8; 2 * 1024 * 1024 + 500 * 1024]; // 2.5MB

        let chunks = chunker.chunk_data(&original_data);
        let reassembled = FileChunker::reassemble_chunks(&chunks).unwrap();

        assert_eq!(original_data, reassembled);
    }

    #[test]
    fn test_content_store() {
        let store = ContentStore::default();
        let test_data = b"Test file content for content addressing";

        let processed = store.process_file(test_data);

        // Verify file hash is consistent
        assert_eq!(processed.file_hash, ContentHash::from_data(test_data));
        assert_eq!(processed.total_size, test_data.len() as u64);

        // Verify chunks are valid
        for chunk in &processed.chunks {
            assert!(store.verify_chunk(chunk));
        }

        // Verify reassembly
        let reassembled = store.verify_and_reassemble(&processed.chunks).unwrap();
        assert_eq!(test_data, reassembled.as_slice());
    }

    #[test]
    fn test_merkle_tree() {
        let chunk_data = vec![
            b"chunk1".as_slice(),
            b"chunk2".as_slice(),
            b"chunk3".as_slice(),
            b"chunk4".as_slice(),
        ];

        let chunk_hashes: Vec<ContentHash> = chunk_data
            .iter()
            .map(|data| ContentHash::from_data(data))
            .collect();

        let tree = MerkleTreeBuilder::build_tree(&chunk_hashes).unwrap();

        // Verify tree structure
        assert_eq!(tree.levels[0].len(), 4); // Leaf level
        assert_eq!(tree.levels[1].len(), 2); // Parent level
        assert_eq!(tree.levels[2].len(), 1); // Root level

        // Test proof generation and verification
        let proof = tree.get_proof(0).unwrap();
        assert!(tree.verify_proof(&proof));
        assert_eq!(proof.chunk_hash, chunk_hashes[0]);

        // Verify proof directly
        assert!(proof.verify());
    }

    #[test]
    fn test_merkle_proof_validation() {
        let chunks = vec![
            ContentHash::from_data(b"test1"),
            ContentHash::from_data(b"test2"),
            ContentHash::from_data(b"test3"),
        ];

        let tree = MerkleTreeBuilder::build_tree(&chunks).unwrap();

        // Valid proof
        let proof = tree.get_proof(1).unwrap();
        assert!(proof.verify());

        // Invalid proof (wrong chunk hash)
        let mut invalid_proof = proof.clone();
        invalid_proof.chunk_hash = ContentHash::from_data(b"wrong");
        assert!(!invalid_proof.verify());
    }
}
