#!/bin/bash

# Carbide Node Repository Cleanup Script
# This script helps clean up build artifacts and other unnecessary files
# to reduce repository disk space usage.

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Flags
DRY_RUN=false
CLEAN_BUILD=false
CLEAN_GIT=false
CLEAN_STORAGE=false
CLEAN_CARGO_CACHE=false
CLEAN_ALL=false

# Function to print colored messages
print_info() {
    echo -e "${BLUE}ℹ${NC} $1"
}

print_success() {
    echo -e "${GREEN}✓${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}⚠${NC} $1"
}

print_error() {
    echo -e "${RED}✗${NC} $1"
}

# Function to get directory size
get_size() {
    if [ -d "$1" ]; then
        du -sh "$1" 2>/dev/null | cut -f1
    else
        echo "0"
    fi
}

# Function to calculate total size before cleanup
calculate_size() {
    local total=0
    local dirs=("$@")
    
    for dir in "${dirs[@]}"; do
        if [ -d "$dir" ]; then
            local size=$(du -sm "$dir" 2>/dev/null | cut -f1)
            total=$((total + size))
        fi
    done
    
    echo "$total"
}

# Function to clean build artifacts
clean_build() {
    print_info "Cleaning build artifacts..."
    
    local dirs=(
        "target"
        "gui/src-tauri/target"
        "gui/dist"
    )
    
    local total_size=0
    for dir in "${dirs[@]}"; do
        if [ -d "$dir" ]; then
            local size=$(du -sm "$dir" 2>/dev/null | cut -f1)
            total_size=$((total_size + size))
            
            if [ "$DRY_RUN" = true ]; then
                print_info "  [DRY RUN] Would remove: $dir ($(get_size "$dir"))"
            else
                print_info "  Removing: $dir ($(get_size "$dir"))"
                rm -rf "$dir"
                print_success "  Removed: $dir"
            fi
        fi
    done
    
    if [ "$DRY_RUN" = true ]; then
        print_info "  [DRY RUN] Would free approximately: ${total_size}MB"
    else
        print_success "Build artifacts cleaned (freed ~${total_size}MB)"
    fi
}

# Function to clean git repository
clean_git() {
    print_info "Cleaning and optimizing git repository..."
    
    if [ "$DRY_RUN" = true ]; then
        local git_size=$(du -sm ".git" 2>/dev/null | cut -f1)
        print_info "  [DRY RUN] Current git size: ${git_size}MB"
        print_info "  [DRY RUN] Would run: git gc --aggressive --prune=now"
        print_warning "  [DRY RUN] This may take several minutes on large repositories"
    else
        print_warning "This may take several minutes on large repositories..."
        git gc --aggressive --prune=now
        local git_size=$(du -sm ".git" 2>/dev/null | cut -f1)
        print_success "Git repository optimized (current size: ${git_size}MB)"
    fi
}

# Function to clean storage directories
clean_storage() {
    print_info "Cleaning storage directories..."
    
    local dirs=(
        "storage"
        "crates/carbide-provider/storage"
    )
    
    local total_size=0
    for dir in "${dirs[@]}"; do
        if [ -d "$dir" ]; then
            local size=$(du -sm "$dir" 2>/dev/null | cut -f1)
            total_size=$((total_size + size))
            
            if [ "$DRY_RUN" = true ]; then
                print_info "  [DRY RUN] Would remove: $dir ($(get_size "$dir"))"
                print_warning "  [DRY RUN] WARNING: This contains runtime/test data!"
            else
                print_warning "  Removing: $dir ($(get_size "$dir"))"
                print_warning "  WARNING: This contains runtime/test data!"
                read -p "  Are you sure you want to delete this? (yes/no): " confirm
                if [ "$confirm" = "yes" ]; then
                    rm -rf "$dir"
                    print_success "  Removed: $dir"
                else
                    print_info "  Skipped: $dir"
                fi
            fi
        fi
    done
    
    if [ "$DRY_RUN" = true ]; then
        print_info "  [DRY RUN] Would free approximately: ${total_size}MB"
    else
        print_success "Storage directories cleaned (freed ~${total_size}MB)"
    fi
}

# Function to clean Cargo cache (system-wide)
clean_cargo_cache() {
    print_info "Cleaning Cargo cache (system-wide)..."
    print_warning "This will affect ALL Rust projects on your system!"
    
    if [ "$DRY_RUN" = true ]; then
        if command -v cargo-cache &> /dev/null; then
            print_info "  [DRY RUN] Would run: cargo cache --autoclean"
        else
            print_info "  [DRY RUN] Would clean: ~/.cargo/registry/cache"
            print_info "  [DRY RUN] Would clean: ~/.cargo/git/db"
            print_warning "  [DRY RUN] Install 'cargo-cache' for better cache management"
        fi
    else
        if command -v cargo-cache &> /dev/null; then
            cargo cache --autoclean
            print_success "Cargo cache cleaned"
        else
            print_warning "cargo-cache not installed. Cleaning manually..."
            if [ -d "$HOME/.cargo/registry/cache" ]; then
                local cache_size=$(du -sm "$HOME/.cargo/registry/cache" 2>/dev/null | cut -f1)
                print_info "  Cargo registry cache: ${cache_size}MB"
                read -p "  Remove Cargo registry cache? (yes/no): " confirm
                if [ "$confirm" = "yes" ]; then
                    rm -rf "$HOME/.cargo/registry/cache"
                    print_success "  Removed Cargo registry cache"
                fi
            fi
            if [ -d "$HOME/.cargo/git/db" ]; then
                local git_size=$(du -sm "$HOME/.cargo/git/db" 2>/dev/null | cut -f1)
                print_info "  Cargo git cache: ${git_size}MB"
                read -p "  Remove Cargo git cache? (yes/no): " confirm
                if [ "$confirm" = "yes" ]; then
                    rm -rf "$HOME/.cargo/git/db"
                    print_success "  Removed Cargo git cache"
                fi
            fi
        fi
    fi
}

# Function to show current repository size
show_current_size() {
    print_info "Current repository size breakdown:"
    echo ""
    
    if [ -d "target" ]; then
        echo "  target/: $(get_size "target")"
    fi
    if [ -d "gui/src-tauri/target" ]; then
        echo "  gui/src-tauri/target/: $(get_size "gui/src-tauri/target")"
    fi
    if [ -d "gui/dist" ]; then
        echo "  gui/dist/: $(get_size "gui/dist")"
    fi
    if [ -d ".git" ]; then
        echo "  .git/: $(get_size ".git")"
    fi
    if [ -d "storage" ]; then
        echo "  storage/: $(get_size "storage")"
    fi
    if [ -d "crates/carbide-provider/storage" ]; then
        echo "  crates/carbide-provider/storage/: $(get_size "crates/carbide-provider/storage")"
    fi
    
    echo ""
    local total_repo_size=$(du -sh . 2>/dev/null | cut -f1)
    echo "  Total repository size: $total_repo_size"
    echo ""
}

# Function to show usage
show_usage() {
    cat << EOF
Usage: $0 [OPTIONS]

Clean up build artifacts and unnecessary files to reduce repository disk space.

OPTIONS:
    --all          Clean everything (build artifacts, git, storage, cargo cache)
    --build        Clean only build artifacts (target directories, dist)
    --git          Clean and optimize git repository
    --storage      Clean storage/test data directories
    --cargo-cache  Clean Cargo cache (system-wide, affects all Rust projects)
    --dry-run      Show what would be deleted without actually deleting
    --help         Show this help message

EXAMPLES:
    $0 --dry-run              # See what would be cleaned
    $0 --build                # Clean only build artifacts
    $0 --all                  # Clean everything
    $0 --build --git          # Clean build artifacts and optimize git

NOTES:
    - Build artifacts can be regenerated with: cargo build --release
    - Tauri GUI can be rebuilt with: cd gui && npm install && npm run tauri:build
    - Storage cleanup requires confirmation as it contains runtime data
    - Cargo cache cleanup affects ALL Rust projects on your system

EOF
}

# Parse command line arguments
if [ $# -eq 0 ]; then
    show_usage
    exit 0
fi

while [[ $# -gt 0 ]]; do
    case $1 in
        --all)
            CLEAN_ALL=true
            shift
            ;;
        --build)
            CLEAN_BUILD=true
            shift
            ;;
        --git)
            CLEAN_GIT=true
            shift
            ;;
        --storage)
            CLEAN_STORAGE=true
            shift
            ;;
        --cargo-cache)
            CLEAN_CARGO_CACHE=true
            shift
            ;;
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        --help)
            show_usage
            exit 0
            ;;
        *)
            print_error "Unknown option: $1"
            show_usage
            exit 1
            ;;
    esac
done

# Main execution
echo ""
print_info "Carbide Node Repository Cleanup"
echo ""

if [ "$DRY_RUN" = true ]; then
    print_warning "DRY RUN MODE - No files will be deleted"
    echo ""
fi

# Show current size
show_current_size

# Execute cleanup based on flags
if [ "$CLEAN_ALL" = true ]; then
    CLEAN_BUILD=true
    CLEAN_GIT=true
    CLEAN_STORAGE=true
    CLEAN_CARGO_CACHE=true
fi

if [ "$CLEAN_BUILD" = true ]; then
    clean_build
    echo ""
fi

if [ "$CLEAN_GIT" = true ]; then
    clean_git
    echo ""
fi

if [ "$CLEAN_STORAGE" = true ]; then
    clean_storage
    echo ""
fi

if [ "$CLEAN_CARGO_CACHE" = true ]; then
    clean_cargo_cache
    echo ""
fi

# Show final size
if [ "$DRY_RUN" = false ]; then
    print_info "Final repository size:"
    show_current_size
    print_success "Cleanup complete!"
else
    print_info "Dry run complete. Run without --dry-run to perform cleanup."
fi

echo ""

