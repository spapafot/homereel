#!/usr/bin/env bash
# Builds Homereel from source and installs it on this Mac (Apple Silicon).
#
# Checks the toolchain (Xcode Command Line Tools, Node.js 24+, Rust) and offers to install what is
# missing. Then it builds the checkout this script is in, or clones the repository to
# $HOMEREEL_DIR (default ~/homereel, pulled on later runs), adds the Google OAuth client, fetches
# ffmpeg, builds Homereel.app and copies it to /Applications. Windows uses scripts/install.ps1.
#
# From a checkout:
#   bash scripts/install.sh [--google-client <path>]
# Without one:
#   curl -fsSL https://raw.githubusercontent.com/spapafot/homereel/main/scripts/install.sh | bash
#   curl -fsSL <same URL> | bash -s -- --google-client <path>
#
# HOMEREEL_GOOGLE_CLIENT can stand in for --google-client.

set -euo pipefail

REPO_URL='https://github.com/spapafot/homereel.git'
MIN_NODE_MAJOR=24
TOOLCHAIN_HELP='See docs/setup.md#toolchain'
# Empty when the script is piped to bash.
SCRIPT_PATH="${BASH_SOURCE[0]:-}"

REPO_ROOT=''
IS_SCRIPT_CHECKOUT=false
HAS_GOOGLE_CLIENT=false

step() {
  printf '\n\033[1;36m==> %s\033[0m\n' "$1"
}

fail() {
  printf '\033[1;31m%s\033[0m\n' "$1" >&2
  exit 1
}

# Reads from the terminal, because stdin is the script itself when it is piped to bash. Without a
# terminal the answer is empty.
ask() {
  local answer=''
  printf '%s ' "$1" >&2
  { read -r answer < /dev/tty; } 2>/dev/null || true
  printf '%s' "$answer"
}

confirm() {
  local answer
  answer="$(ask "$1 [Y/n]")"
  [[ ! "$answer" =~ ^[[:space:]]*[Nn] ]]
}

node_major() {
  if ! command -v node >/dev/null 2>&1; then
    echo 0
    return
  fi
  node -p 'process.versions.node.split(".")[0]'
}

# The Command Line Tools also provide git.
ensure_command_line_tools() {
  if xcode-select -p >/dev/null 2>&1; then
    return
  fi
  step 'Installing the Xcode Command Line Tools'
  xcode-select --install >/dev/null 2>&1 || true
  echo 'Click Install in the window that opened. Waiting for the installation to finish...'
  until xcode-select -p >/dev/null 2>&1; do
    sleep 10
  done
}

ensure_node() {
  if (( $(node_major) >= MIN_NODE_MAJOR )); then
    return
  fi
  local missing="Node.js $MIN_NODE_MAJOR or later"
  if ! command -v brew >/dev/null 2>&1; then
    fail "Homereel needs $missing. Install it from https://nodejs.org and run this again."
  fi
  confirm "$missing is missing. Install it with Homebrew?" ||
    fail "Install $missing and run this again. $TOOLCHAIN_HELP."
  step 'Installing Node.js'
  brew install node
  hash -r
  if (( $(node_major) < MIN_NODE_MAJOR )); then
    fail "The node on the PATH is still older than $MIN_NODE_MAJOR. Open a new terminal and retry."
  fi
}

ensure_rust() {
  if [[ -f "$HOME/.cargo/env" ]]; then
    # shellcheck source=/dev/null
    source "$HOME/.cargo/env"
  fi
  if command -v cargo >/dev/null 2>&1; then
    return
  fi
  confirm 'Rust is missing. Install it with rustup (https://rustup.rs)?' ||
    fail "Install Rust and run this again. $TOOLCHAIN_HELP."
  step 'Installing Rust'
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
  # shellcheck source=/dev/null
  source "$HOME/.cargo/env"
}

get_source() {
  if [[ -n "$SCRIPT_PATH" && -f "$SCRIPT_PATH" ]]; then
    local checkout
    checkout="$(cd "$(dirname "$SCRIPT_PATH")/.." && pwd)"
    if [[ -f "$checkout/src-tauri/tauri.conf.json" ]]; then
      REPO_ROOT="$checkout"
      IS_SCRIPT_CHECKOUT=true
      step "Building the checkout at $REPO_ROOT"
      return
    fi
  fi

  REPO_ROOT="${HOMEREEL_DIR:-$HOME/homereel}"
  if [[ -f "$REPO_ROOT/src-tauri/tauri.conf.json" ]]; then
    step "Updating $REPO_ROOT"
    git -C "$REPO_ROOT" pull --ff-only
  elif [[ -e "$REPO_ROOT" && -n "$(ls -A "$REPO_ROOT")" ]]; then
    fail "$REPO_ROOT exists and is not a Homereel checkout. Set HOMEREEL_DIR to another folder."
  else
    step "Cloning $REPO_URL to $REPO_ROOT"
    git clone "$REPO_URL" "$REPO_ROOT"
  fi
}

add_google_client() {
  local client_file="$1"
  local target="$REPO_ROOT/src-tauri/google-oauth-client.json"
  if [[ -f "$target" || -n "${HOMEREEL_GOOGLE_CLIENT_ID:-}" ]]; then
    HAS_GOOGLE_CLIENT=true
  fi

  if [[ -z "$client_file" && "$HAS_GOOGLE_CLIENT" == false ]]; then
    step 'Google Drive access'
    echo 'Homereel reads Google Drive with your own OAuth client. docs/setup.md shows how to'
    echo 'create one (about 10 minutes). Videos in folders on this Mac work without it.'
    client_file="$(ask 'Drag its JSON file here, or press Enter to build without Google Drive:')"
    # Terminal escapes spaces in a dragged-in path with backslashes and adds a space after it.
    local backslash='\'
    client_file="${client_file%"${client_file##*[![:space:]]}"}"
    client_file="${client_file//"$backslash"/}"
    client_file="${client_file/#\~/$HOME}"
  fi
  if [[ -z "$client_file" ]]; then
    return
  fi

  [[ -f "$client_file" ]] || fail "Google OAuth client file not found: $client_file"
  local is_desktop_client='
    const client = JSON.parse(require("fs").readFileSync(process.argv[1], "utf8"));
    process.exit(client.installed?.client_id ? 0 : 1);'
  node -e "$is_desktop_client" "$client_file" 2>/dev/null ||
    fail "$client_file is not a Desktop app OAuth client. Create one as described in docs/setup.md."
  if [[ ! "$client_file" -ef "$target" ]]; then
    cp "$client_file" "$target"
  fi
  echo "Using the Google OAuth client from $client_file"
  HAS_GOOGLE_CLIENT=true
}

build_app() {
  cd "$REPO_ROOT"
  step 'Installing npm packages'
  npm ci
  step 'Fetching ffmpeg'
  bash scripts/fetch-ffmpeg.sh
  step 'Building Homereel (the first build takes several minutes)'
  # Only the app: installing does not need the disk image, whose window layout needs Finder.
  npm run build -- --bundles app
}

install_app() {
  local built="$REPO_ROOT/src-tauri/target/release/bundle/macos/Homereel.app"
  local applications='/Applications'
  if [[ ! -w "$applications" ]]; then
    applications="$HOME/Applications"
    mkdir -p "$applications"
  fi
  local installed="$applications/Homereel.app"

  local was_running=false
  if pgrep -f "$installed/Contents/MacOS/" >/dev/null; then
    was_running=true
  fi

  step "Installing to $installed"
  rm -rf "$installed"
  ditto "$built" "$installed"

  printf '\n\033[1;32m%s\033[0m\n' 'Homereel is installed. Open it from Launchpad or Spotlight.'
  if [[ "$was_running" == true ]]; then
    echo 'Quit and reopen Homereel to use the new version.'
  fi
  echo 'After an update, macOS may ask whether Homereel may use the keychain. Enter your password'
  echo 'and click Always Allow.'
}

main() {
  local google_client="${HOMEREEL_GOOGLE_CLIENT:-}"
  while (( $# > 0 )); do
    case "$1" in
      --google-client)
        [[ $# -ge 2 ]] || fail '--google-client needs the path to the OAuth client JSON.'
        google_client="$2"
        shift 2
        ;;
      *)
        fail "Unknown option: $1. Usage: install.sh [--google-client <path>]"
        ;;
    esac
  done

  if [[ "$(uname -s)" != 'Darwin' ]]; then
    fail 'This script is for macOS. On Windows, use scripts/install.ps1.'
  fi
  if [[ "$(uname -m)" != 'arm64' ]]; then
    fail 'Homereel builds for Apple Silicon Macs only.'
  fi
  if ! command -v brew >/dev/null 2>&1 && [[ -x /opt/homebrew/bin/brew ]]; then
    eval "$(/opt/homebrew/bin/brew shellenv)"
  fi

  ensure_command_line_tools
  ensure_node
  ensure_rust
  get_source
  add_google_client "$google_client"
  build_app
  install_app

  if [[ "$HAS_GOOGLE_CLIENT" == false ]]; then
    printf '\033[1;33m%s %s\033[0m\n' 'This build cannot connect to Google Drive.' \
      'Create the OAuth client (docs/setup.md) and run this again.'
  fi
  if [[ "$IS_SCRIPT_CHECKOUT" == false ]]; then
    echo "To update, run the same command again. The source stays in $REPO_ROOT."
  fi
}

# Everything runs from main, so bash has read the whole script before a command can read stdin.
main "$@"
