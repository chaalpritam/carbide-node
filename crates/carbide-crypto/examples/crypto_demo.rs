//! # Carbide Crypto Demo
//!
//! Demonstrates the cryptographic capabilities including:
//! - File chunking and content addressing
//! - AES-256-GCM encryption/decryption
//! - Merkle tree construction and verification
//! - Proof-of-storage challenges and responses

use carbide_core::ContentHash;
use carbide_crypto::{MIN_CHUNK_SIZE, *};

fn main() -> carbide_core::Result<()> {
    println!("🔐 Carbide Crypto System Demo");
    println!("=============================\n");

    // 1. Demonstrate file chunking and content addressing
    demo_file_chunking()?;

    // 2. Demonstrate encryption and decryption
    demo_encryption()?;

    // 3. Demonstrate Merkle trees
    demo_merkle_trees()?;

    // 4. Demonstrate proof-of-storage
    demo_proof_of_storage()?;

    println!("\n✅ Crypto demo completed! This shows how Carbide Network ensures:");
    println!("   • Content integrity through BLAKE3 hashing");
    println!("   • Data privacy through AES-256-GCM encryption");
    println!("   • Efficient verification using Merkle trees");
    println!("   • Provider accountability via proof-of-storage");

    Ok(())
}

fn demo_file_chunking() -> carbide_core::Result<()> {
    println!("📦 File Chunking and Content Addressing");
    println!("---------------------------------------");

    // Create test files of different sizes
    let small_file_data = b"Hello, Carbide Network! This is a small file.";
    let large_file_data = vec![42u8; 2 * 1024 * 1024 + 500 * 1024]; // 2.5MB file

    // Process files through content store
    let store = ContentStore::default();

    // Small file (single chunk)
    let small_processed = store.process_file(small_file_data);
    println!("📄 Small file ({} bytes):", small_file_data.len());
    println!("   Content hash: {}", small_processed.file_hash);
    println!("   Chunks: {}", small_processed.chunk_count());
    println!("   Single chunk: {}", small_processed.is_single_chunk());

    // Large file (multiple chunks with 1MB chunker)
    let chunker = FileChunker::new(MIN_CHUNK_SIZE)?; // 1MB chunks
    let custom_store = ContentStore::new(chunker);
    let large_processed = custom_store.process_file(&large_file_data);

    println!(
        "\n📄 Large file ({:.1} MB):",
        large_file_data.len() as f64 / (1024.0 * 1024.0)
    );
    println!("   Content hash: {}", large_processed.file_hash);
    println!("   Chunks: {}", large_processed.chunk_count());
    println!("   Single chunk: {}", large_processed.is_single_chunk());

    // Verify chunk integrity and reassembly
    let reassembled = custom_store.verify_and_reassemble(&large_processed.chunks)?;
    println!(
        "   Integrity check: ✅ {} bytes reassembled correctly",
        reassembled.len()
    );

    // Show chunk details
    println!("\n   Chunk details:");
    for (i, chunk) in large_processed.chunks.iter().take(3).enumerate() {
        println!(
            "     Chunk {}: {} bytes at offset {}, hash: {}...",
            i,
            chunk.data.len(),
            chunk.offset,
            &chunk.hash.to_hex()[..16]
        );
    }
    if large_processed.chunks.len() > 3 {
        println!(
            "     ... and {} more chunks",
            large_processed.chunks.len() - 3
        );
    }

    println!();
    Ok(())
}

fn demo_encryption() -> carbide_core::Result<()> {
    println!("🔒 File Encryption and Key Management");
    println!("-----------------------------------");

    let test_data = b"This is sensitive data that needs to be encrypted before storage.";
    println!(
        "📄 Original data ({} bytes): \"{}\"",
        test_data.len(),
        String::from_utf8_lossy(test_data)
    );

    // 1. Basic encryption/decryption
    println!("\n🔑 Basic Encryption:");
    let encryption_key = EncryptionKey::generate()?;
    let encryptor = FileEncryptor::new(&encryption_key)?;
    let decryptor = FileDecryptor::new(&encryption_key)?;

    let encrypted_data = encryptor.encrypt(test_data)?;
    println!(
        "   Encrypted size: {} bytes (includes 16-byte auth tag)",
        encrypted_data.ciphertext.len()
    );
    println!("   Nonce: {}", encrypted_data.nonce.to_hex());
    println!(
        "   Ciphertext: {}...",
        hex::encode(&encrypted_data.ciphertext[..16])
    );

    let decrypted_data = decryptor.decrypt(&encrypted_data)?;
    println!(
        "   Decrypted: \"{}\"",
        String::from_utf8_lossy(&decrypted_data)
    );
    println!("   ✅ Encryption roundtrip successful");

    // 2. Password-based key derivation
    println!("\n🔑 Password-Based Key Derivation:");
    let password = "my_secure_password_123";
    let salt = KeyDerivation::generate_salt()?;

    let derived_key = KeyDerivation::derive_from_password(password, &salt, 10000)?;
    println!("   Password: \"{}\"", password);
    println!("   Salt: {}...", hex::encode(&salt[..8]));
    println!("   Iterations: 10,000");
    println!("   Derived key: {}...", &derived_key.to_hex()[..16]);

    // 3. Master key management
    println!("\n🔑 Hierarchical Key Management:");
    let key_manager = KeyManager::generate()?;

    let file1_key = key_manager.derive_file_key("document1.pdf")?;
    let file2_key = key_manager.derive_file_key("video.mp4")?;
    let file1_key_again = key_manager.derive_file_key("document1.pdf")?;

    println!("   File 1 key: {}...", &file1_key.to_hex()[..16]);
    println!("   File 2 key: {}...", &file2_key.to_hex()[..16]);
    println!("   File 1 again: {}...", &file1_key_again.to_hex()[..16]);
    println!(
        "   ✅ Same file always gets same key: {}",
        file1_key.as_bytes() == file1_key_again.as_bytes()
    );
    println!(
        "   ✅ Different files get different keys: {}",
        file1_key.as_bytes() != file2_key.as_bytes()
    );

    // 4. Encrypted key export/import
    println!("\n🔑 Secure Key Backup:");
    let export_password = "backup_password_456";
    let encrypted_master = key_manager.export_encrypted_master_key(export_password)?;
    println!("   Master key encrypted with password");
    println!("   Salt: {}...", hex::encode(&encrypted_master.salt[..8]));
    println!("   Iterations: {}", encrypted_master.iterations);

    let restored_manager =
        KeyManager::import_encrypted_master_key(&encrypted_master, export_password)?;
    let restored_file1_key = restored_manager.derive_file_key("document1.pdf")?;

    println!(
        "   ✅ Key restoration successful: {}",
        file1_key.as_bytes() == restored_file1_key.as_bytes()
    );

    println!();
    Ok(())
}

fn demo_merkle_trees() -> carbide_core::Result<()> {
    println!("🌳 Merkle Tree Construction and Proofs");
    println!("-------------------------------------");

    // Create some test chunks
    let chunk_data = vec![
        "First chunk of the file data",
        "Second chunk continues here",
        "Third chunk has more content",
        "Fourth chunk completes the file",
        "Fifth chunk for odd number test",
    ];

    let chunk_hashes: Vec<ContentHash> = chunk_data
        .iter()
        .map(|data| ContentHash::from_data(data.as_bytes()))
        .collect();

    println!("📄 File with {} chunks:", chunk_hashes.len());
    for (i, hash) in chunk_hashes.iter().enumerate() {
        println!("   Chunk {}: {}...", i, &hash.to_hex()[..16]);
    }

    // Build Merkle tree
    let tree = MerkleTreeBuilder::build_tree(&chunk_hashes)?;
    println!("\n🌳 Merkle Tree:");
    println!("   Root hash: {}...", &tree.root_hash().to_hex()[..16]);
    println!("   Tree levels: {}", tree.levels.len());
    for (level, hashes) in tree.levels.iter().enumerate() {
        println!("     Level {}: {} nodes", level, hashes.len());
    }

    // Generate and verify proofs
    println!("\n🔍 Merkle Proofs:");
    for chunk_index in [0, 2, 4] {
        let proof = tree.get_proof(chunk_index)?;
        let is_valid = tree.verify_proof(&proof);

        println!("   Chunk {} proof:", chunk_index);
        println!("     Proof path length: {}", proof.proof_hashes.len());
        println!(
            "     Verification: {} {}",
            if is_valid { "✅" } else { "❌" },
            if is_valid { "Valid" } else { "Invalid" }
        );
    }

    // Show proof efficiency
    let proof_size = tree.get_proof(0)?.proof_hashes.len() * 32; // 32 bytes per hash
    let full_data_size: usize = chunk_data.iter().map(|s| s.len()).sum();
    println!("\n📊 Proof Efficiency:");
    println!("   Full data size: {} bytes", full_data_size);
    println!("   Proof size: {} bytes", proof_size);
    println!(
        "   Efficiency: {:.1}x smaller",
        full_data_size as f32 / proof_size as f32
    );

    println!();
    Ok(())
}

fn demo_proof_of_storage() -> carbide_core::Result<()> {
    println!("🛡️ Proof-of-Storage Challenges");
    println!("------------------------------");

    let file_hash = ContentHash::from_data(b"Important file that needs proof of storage");
    let total_chunks = 20;

    println!("📄 File to verify:");
    println!("   Hash: {}...", &file_hash.to_hex()[..16]);
    println!("   Total chunks: {}", total_chunks);

    // Generate storage challenge
    let generator = ChallengeGenerator::new();
    let challenge = generator.generate_challenge(
        file_hash,
        total_chunks,
        0.3, // Challenge 30% of chunks
    )?;

    println!("\n🎯 Storage Challenge:");
    println!("   Challenge ID: {}", challenge.challenge_id);
    println!(
        "   Chunks to prove: {} ({:.0}%)",
        challenge.chunk_indices.len(),
        challenge.chunk_indices.len() as f32 / total_chunks as f32 * 100.0
    );
    println!("   Challenge indices: {:?}", challenge.chunk_indices);
    println!("   Expires at: {}", challenge.expires_at.format("%H:%M:%S"));
    println!(
        "   Expected response: {}...",
        &challenge.expected_response_hash.to_hex()[..16]
    );

    // Simulate provider response (normally they would provide real Merkle proofs)
    println!("\n📋 Provider Response:");
    println!("   Provider must prove possession of specific chunks");
    println!("   Each proof includes:");
    println!("     • Chunk hash for integrity verification");
    println!("     • Merkle path to file root hash");
    println!("     • Digital signature for authenticity");

    // Show challenge statistics
    let mut proof_manager = ProofManager::new();
    proof_manager.issue_challenge(file_hash, total_chunks, 0.25)?;
    proof_manager.issue_challenge(file_hash, 15, 0.4)?;
    proof_manager.issue_challenge(file_hash, 8, 0.5)?;

    let stats = proof_manager.get_statistics();
    println!("\n📊 Proof Manager Statistics:");
    println!("   Active challenges: {}", stats.active_challenges);
    println!("   Completed proofs: {}", stats.completed_proofs);
    println!("   Expired challenges: {}", stats.expired_challenges);

    // Explain security benefits
    println!("\n🔒 Security Benefits:");
    println!("   • Providers cannot fake storage without the actual data");
    println!("   • Random challenges prevent pre-computation attacks");
    println!("   • Merkle proofs enable efficient verification");
    println!("   • Time limits prevent delayed responses");
    println!("   • Cryptographic signatures ensure authenticity");

    println!();
    Ok(())
}
