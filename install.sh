#!/bin/sh
set -eu

# cq installer — downloads the latest release binary from GitHub
# Usage: curl -fsSL https://raw.githubusercontent.com/OWNER/cq/main/install.sh | sh

REPO="bbbenja/cq"
INSTALL_DIR="${CQ_INSTALL_DIR:-$HOME/.local/bin}"

main() {
    os="$(uname -s)"
    arch="$(uname -m)"

    case "$os" in
        Linux)  os_target="unknown-linux-gnu" ;;
        Darwin) os_target="apple-darwin" ;;
        *)      err "Unsupported OS: $os" ;;
    esac

    case "$arch" in
        x86_64|amd64)   arch_target="x86_64" ;;
        aarch64|arm64)  arch_target="aarch64" ;;
        *)              err "Unsupported architecture: $arch" ;;
    esac

    target="${arch_target}-${os_target}"

    # Get latest tag
    tag="$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
        | grep '"tag_name"' | head -1 | cut -d'"' -f4)"

    if [ -z "$tag" ]; then
        err "Could not determine latest release"
    fi

    archive="cq-${tag}-${target}.tar.gz"
    url="https://github.com/${REPO}/releases/download/${tag}/${archive}"

    echo "Installing cq ${tag} (${target})..."

    tmpdir="$(mktemp -d)"
    trap 'rm -rf "$tmpdir"' EXIT

    curl -fsSL "$url" -o "${tmpdir}/${archive}"
    tar xzf "${tmpdir}/${archive}" -C "$tmpdir"

    mkdir -p "$INSTALL_DIR"
    mv "${tmpdir}/cq" "${INSTALL_DIR}/cq"
    chmod +x "${INSTALL_DIR}/cq"

    echo "Installed cq to ${INSTALL_DIR}/cq"

    if ! echo "$PATH" | tr ':' '\n' | grep -qx "$INSTALL_DIR"; then
        echo ""
        echo "Add ${INSTALL_DIR} to your PATH:"
        echo "  export PATH=\"${INSTALL_DIR}:\$PATH\""
    fi

    echo ""
    echo "Run 'cq install' to set up the git commit alias."
}

err() {
    echo "Error: $1" >&2
    exit 1
}

main
