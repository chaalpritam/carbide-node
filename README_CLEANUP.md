# Repository Cleanup Guide

This guide explains how to clean up your Carbide Node repository to reduce disk space usage. The repository can grow to **~12GB** due to build artifacts, caches, and other generated files.

## Quick Start

```bash
# See what would be cleaned (recommended first step)
./clean.sh --dry-run

# Clean build artifacts only (safest option)
./clean.sh --build

# Clean everything
./clean.sh --all
```

## What Can Be Safely Cleaned

### ✅ Build Artifacts (Safe to Clean - ~9.2GB)

These directories contain compiled binaries and intermediate build files that can be regenerated:

- **`target/`** (~7.1GB) - Rust build artifacts (debug and release builds)
- **`gui/src-tauri/target/`** (~2.1GB) - Tauri desktop app build artifacts
- **`gui/dist/`** (~604KB) - Frontend build output (Vite)

**Safety**: ✅ Completely safe - all files are in `.gitignore` and can be rebuilt.

**Rebuild commands**:
```bash
# Rebuild Rust binaries
cargo build --release

# Rebuild Tauri GUI
cd gui
npm install
npm run tauri:build
```

### ⚠️ Git Repository (Optional - Variable Size)

The `.git/` directory contains version history and can be optimized:

- Large pack files from git history
- Unreachable objects and refs

**Safety**: ⚠️ Generally safe, but optimizes repository history. Use `git gc --aggressive --prune=now`.

**Note**: This operation may take several minutes on large repositories.

### ⚠️ Storage Directories (Use with Caution)

Runtime and test data directories:

- **`storage/`** - Runtime storage data
- **`crates/carbide-provider/storage/`** - Provider storage data

**Safety**: ⚠️ Contains runtime/test data. Only clean if you're sure it's test data you don't need.

### ⚠️ Cargo Cache (System-Wide - Use with Caution)

Cargo cache is shared across all Rust projects:

- **`~/.cargo/registry/cache/`** - Cached crate downloads
- **`~/.cargo/git/db/`** - Git-based dependency cache

**Safety**: ⚠️ Affects ALL Rust projects on your system. Cargo will re-download crates as needed, but this may slow down builds temporarily.

**Note**: Consider installing `cargo-cache` for better cache management:
```bash
cargo install cargo-cache
```

## Cleanup Script Options

The `clean.sh` script provides several options:

| Option | Description |
|--------|-------------|
| `--all` | Clean everything (build artifacts, git, storage, cargo cache) |
| `--build` | Clean only build artifacts (target directories, dist) |
| `--git` | Clean and optimize git repository |
| `--storage` | Clean storage/test data directories |
| `--cargo-cache` | Clean Cargo cache (system-wide) |
| `--dry-run` | Show what would be deleted without actually deleting |
| `--help` | Show help message |

## Examples

### Example 1: Preview Cleanup (Recommended First Step)

```bash
./clean.sh --dry-run
```

This shows what would be cleaned without actually deleting anything.

### Example 2: Clean Build Artifacts Only

```bash
./clean.sh --build
```

This is the safest option and will free approximately **9.2GB** of space.

### Example 3: Clean Build Artifacts and Optimize Git

```bash
./clean.sh --build --git
```

Cleans build artifacts and optimizes the git repository.

### Example 4: Full Cleanup

```bash
./clean.sh --all
```

Cleans everything. You'll be prompted for confirmation on storage and cargo cache cleanup.

## Expected Space Savings

| Cleanup Type | Approximate Savings |
|--------------|---------------------|
| Build artifacts (`--build`) | ~9.2GB |
| Git optimization (`--git`) | Variable (depends on history) |
| Storage directories (`--storage`) | Variable (usually small) |
| Cargo cache (`--cargo-cache`) | Variable (system-wide) |
| **Total (build artifacts only)** | **~9.2GB** |

## Manual Cleanup (Alternative to Script)

If you prefer to clean manually:

```bash
# Clean Rust build artifacts
cargo clean
rm -rf target/
rm -rf gui/src-tauri/target/

# Clean frontend build
rm -rf gui/dist/

# Optimize git (optional)
git gc --aggressive --prune=now

# Clean Cargo cache (optional, system-wide)
# Requires cargo-cache: cargo install cargo-cache
cargo cache --autoclean
```

## After Cleanup

### Rebuilding the Project

After cleaning build artifacts, you'll need to rebuild:

```bash
# 1. Rebuild Rust binaries
cargo build --release

# 2. Rebuild Tauri GUI (if needed)
cd gui
npm install
npm run tauri:build
```

### Build Times

- **Rust binaries**: 10-15 minutes (first build after cleanup)
- **Tauri GUI**: 5-10 minutes (first build after cleanup)
- Subsequent builds will be faster due to incremental compilation

## Troubleshooting

### Script Permission Denied

If you get a permission error:
```bash
chmod +x clean.sh
```

### Cargo Cache Not Found

If `cargo-cache` is not installed, the script will offer manual cleanup options for Cargo cache directories.

### Storage Cleanup Requires Confirmation

Storage cleanup requires explicit confirmation because it contains runtime data. Make sure you understand what you're deleting.

## Best Practices

1. **Always run `--dry-run` first** to see what will be cleaned
2. **Start with `--build`** for the safest cleanup
3. **Clean regularly** to prevent repository from growing too large
4. **Keep git history** unless you specifically want to optimize it
5. **Be cautious with storage cleanup** - only if you're sure it's test data

## What's NOT Cleaned

The cleanup script does NOT remove:
- Source code files (`.rs`, `.tsx`, `.ts`, etc.)
- Configuration files (`Cargo.toml`, `package.json`, etc.)
- Documentation files (`.md` files)
- Git-tracked files (only `.gitignore`-listed files are cleaned)

## Related Files

- `.gitignore` - Lists all files/directories that are ignored by git (and safe to clean)
- `clean.sh` - The cleanup script
- `Cargo.toml` - Rust project configuration
- `gui/package.json` - Frontend project configuration

## Questions?

If you have questions about what can be safely cleaned, check:
1. `.gitignore` - If it's listed there, it's safe to clean
2. Run `./clean.sh --dry-run` to preview cleanup
3. Start with `--build` flag for the safest cleanup option

