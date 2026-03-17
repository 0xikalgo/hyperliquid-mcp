#!/bin/sh
set -e

REPO="0xikalgo/hyperliquid-mcp"
BINARY="hyperliquid-mcp"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Linux)  os="linux" ;;
  Darwin) os="macos" ;;
  *)      echo "Unsupported OS: $OS"; exit 1 ;;
esac

case "$ARCH" in
  x86_64|amd64)  arch="x86_64" ;;
  aarch64|arm64) arch="aarch64" ;;
  *)             echo "Unsupported architecture: $ARCH"; exit 1 ;;
esac

ARTIFACT="${BINARY}-${os}-${arch}"

# Get latest version if not specified
if [ -z "$VERSION" ]; then
  VERSION="$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | cut -d'"' -f4)"
  if [ -z "$VERSION" ]; then
    echo "Failed to fetch latest version"
    exit 1
  fi
fi

URL="https://github.com/${REPO}/releases/download/${VERSION}/${ARTIFACT}.tar.gz"

echo "Installing ${BINARY} ${VERSION} (${os}/${arch})..."
echo "  From: ${URL}"
echo "  To:   ${INSTALL_DIR}/${BINARY}"

TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

curl -fsSL "$URL" -o "${TMPDIR}/${ARTIFACT}.tar.gz"
tar -xzf "${TMPDIR}/${ARTIFACT}.tar.gz" -C "$TMPDIR"

mkdir -p "$INSTALL_DIR"
mv "${TMPDIR}/${ARTIFACT}" "${INSTALL_DIR}/${BINARY}"
chmod +x "${INSTALL_DIR}/${BINARY}"

echo ""
echo "Installed ${BINARY} to ${INSTALL_DIR}/${BINARY}"

case ":$PATH:" in
  *":${INSTALL_DIR}:"*) ;;
  *)
    echo ""
    PARENT_SHELL="$(ps -o comm= -p "$PPID" 2>/dev/null || echo "")"
    case "$PARENT_SHELL" in
      *fish)
        echo "NOTE: ${INSTALL_DIR} is not in your PATH. Add it with:"
        echo "  fish_add_path ${INSTALL_DIR}"
        ;;
      *)
        echo "NOTE: ${INSTALL_DIR} is not in your PATH. Add it with:"
        echo "  export PATH=\"${INSTALL_DIR}:\$PATH\""
        ;;
    esac
    ;;
esac
