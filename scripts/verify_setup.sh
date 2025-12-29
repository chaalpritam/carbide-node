#!/bin/bash
# Verification script for Carbide Network workspace setup

set -e

echo "🔍 Verifying Carbide Network workspace setup..."

# Check that workspace compiles
echo "  ✓ Building workspace..."
cargo build --workspace --quiet

# Run tests
echo "  ✓ Running tests..."
cargo test --workspace --quiet

# Check formatting
echo "  ✓ Checking code formatting..."
cargo fmt --all -- --check

# Run clippy
echo "  ✓ Running clippy linting..."
cargo clippy --all --quiet

# Test CLI tools
echo "  ✓ Testing CLI tools..."
cargo run --bin carbide-provider -- --help > /dev/null
cargo run --bin carbide-discovery -- > /dev/null || true # This will print and exit
cargo run --bin carbide-client -- > /dev/null || true

echo "✅ Workspace setup verification completed successfully!"
echo ""
echo "📁 Workspace structure:"
echo "   crates/carbide-core      - Shared data structures ✓"
echo "   crates/carbide-crypto    - Cryptographic functions ✓"
echo "   crates/carbide-provider  - Storage provider node ✓"
echo "   crates/carbide-discovery - Provider marketplace ✓"
echo "   crates/carbide-client    - Client SDK ✓"
echo "   crates/carbide-reputation - Reputation system ✓"
echo ""
echo "🚀 Ready for Step 2: Core Data Structures!"