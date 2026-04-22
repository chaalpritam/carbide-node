#!/usr/bin/env bash
# Carbide Network - Homebrew-based Mac mini storage provider installer.
#
# Installs Homebrew (if missing), adds the chaalpritam/carbide tap,
# builds carbide-node from source, writes a default provider config,
# and starts the provider under launchd via `brew services`.
#
# Usage:  curl -fsSL https://raw.githubusercontent.com/chaalpritam/homebrew-carbide/master/install.sh | bash
#    or:  bash scripts/brew-install.sh

set -euo pipefail

TAP="chaalpritam/carbide"
TAP_URL="https://github.com/chaalpritam/homebrew-carbide"
FORMULA="carbide-node"

RED=$'\033[0;31m'
GREEN=$'\033[0;32m'
YELLOW=$'\033[1;33m'
BLUE=$'\033[0;34m'
NC=$'\033[0m'

log()   { printf '%s▶%s %s\n' "$BLUE"  "$NC" "$*"; }
ok()    { printf '%s✓%s %s\n' "$GREEN" "$NC" "$*"; }
warn()  { printf '%s!%s %s\n' "$YELLOW" "$NC" "$*"; }
die()   { printf '%s✗%s %s\n' "$RED"   "$NC" "$*" >&2; exit 1; }

require_macos() {
  [[ "$(uname)" == "Darwin" ]] || die "This installer only supports macOS."
}

ensure_brew() {
  if command -v brew >/dev/null 2>&1; then
    ok "Homebrew present: $(brew --version | head -1)"
    return
  fi
  log "Installing Homebrew..."
  NONINTERACTIVE=1 /bin/bash -c \
    "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

  if [[ -d /opt/homebrew/bin ]]; then
    eval "$(/opt/homebrew/bin/brew shellenv)"
  elif [[ -d /usr/local/bin ]]; then
    eval "$(/usr/local/bin/brew shellenv)"
  fi
  ok "Homebrew installed"
}

ensure_tap() {
  if brew tap | grep -qx "$TAP"; then
    ok "Tap $TAP already present"
  else
    log "Adding tap $TAP..."
    brew tap "$TAP" "$TAP_URL"
    ok "Tap added"
  fi
}

install_formula() {
  log "Installing $FORMULA (building from source, this may take several minutes)..."
  if brew list --formula | grep -qx "$FORMULA"; then
    warn "$FORMULA already installed - upgrading"
    brew upgrade --fetch-HEAD "$TAP/$FORMULA" || brew reinstall --HEAD "$TAP/$FORMULA"
  else
    brew install --HEAD "$TAP/$FORMULA"
  fi
  ok "$FORMULA installed: $(brew --prefix)/bin/carbide-provider"
}

start_service() {
  log "Starting provider as a launchd service..."
  brew services start "$FORMULA"
  ok "Service started. Manage with: brew services (start|stop|restart) $FORMULA"
}

print_summary() {
  prefix="$(brew --prefix)"
  cat <<EOF

${GREEN}Carbide Network provider node is live on this Mac.${NC}

Configuration:   ${prefix}/etc/carbide/provider.toml
Storage:         ${prefix}/var/carbide/storage
Logs:            ${prefix}/var/log/carbide/

Next steps:
  1. Edit ${prefix}/etc/carbide/provider.toml to set your
     storage allocation, price, and region.
  2. Apply changes:  brew services restart ${FORMULA}
  3. Check status:   carbide-provider status --endpoint http://localhost:8080

EOF
}

main() {
  require_macos
  ensure_brew
  ensure_tap
  install_formula
  start_service
  print_summary
}

main "$@"
