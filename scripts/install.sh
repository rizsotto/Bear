#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-or-later
#
# Install script for Bear.
#
# Environment variables:
#   DESTDIR          - staging directory prepended to all paths (default: empty)
#   PREFIX           - installation prefix (default: /usr/local if root, $HOME/.local otherwise)
#   INTERCEPT_LIBDIR - library directory name (default: lib)
#   SRCDIR           - directory containing build artifacts (default: target/release)
#
# Usage:
#   ./scripts/install.sh
#
#   PREFIX=/usr INTERCEPT_LIBDIR=lib64 ./scripts/install.sh
#   DESTDIR=/tmp/staging PREFIX=/usr ./scripts/install.sh

set -eux

# --- configuration -----------------------------------------------------------

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

PREFIX="${PREFIX:-}"
if [ -z "$PREFIX" ]; then
    if [ "$(id -u)" -eq 0 ]; then
        PREFIX="/usr/local"
    else
        PREFIX="$HOME/.local"
    fi
fi

# Resolve to absolute path
PREFIX="$(cd "$PREFIX" 2>/dev/null && pwd || echo "$PREFIX")"

INTERCEPT_LIBDIR="${INTERCEPT_LIBDIR-lib}"
DESTDIR="${DESTDIR:-}"

# Strip trailing slash from DESTDIR to avoid double slashes
DESTDIR="${DESTDIR%/}"

# --- safety guards ------------------------------------------------------------

if [ "$PREFIX" = "/" ]; then
    echo "error: refusing to operate with PREFIX=/ (would clobber the root filesystem)" >&2
    exit 1
fi

if [ -n "$DESTDIR" ]; then
    case "$DESTDIR" in
        /*) ;;
        *)
            echo "error: DESTDIR must be an absolute path, got: $DESTDIR" >&2
            exit 1
            ;;
    esac
fi

# Reject empty or whitespace-only INTERCEPT_LIBDIR
if [ -z "$(echo "$INTERCEPT_LIBDIR" | tr -d '[:space:]')" ]; then
    echo "error: INTERCEPT_LIBDIR must not be empty or whitespace-only" >&2
    exit 1
fi
# Reject absolute paths
case "$INTERCEPT_LIBDIR" in
    /*)
        echo "error: INTERCEPT_LIBDIR must be a relative path, got: $INTERCEPT_LIBDIR" >&2
        exit 1
        ;;
esac

# --- artifact discovery -------------------------------------------------------

if [ -n "${SRCDIR:-}" ]; then
    SRCDIR="$SRCDIR"
elif [ -d "$REPO_ROOT/target/release" ]; then
    SRCDIR="$REPO_ROOT/target/release"
else
    echo "error: cannot find build artifacts in target/release/" >&2
    exit 1
fi

# --- platform detection -------------------------------------------------------

OS="$(uname -s)"
case "$OS" in
    Linux|FreeBSD|NetBSD|OpenBSD|DragonFly)
        PRELOAD_NAME="libexec.so"
        HAS_PRELOAD=true
        ;;
    Darwin)
        PRELOAD_NAME="libexec.dylib"
        HAS_PRELOAD=true
        ;;
    *)
        PRELOAD_NAME=""
        HAS_PRELOAD=false
        ;;
esac

# --- install ------------------------------------------------------------------

# bear-driver and bear-wrapper
mkdir -p "$DESTDIR$PREFIX/libexec/bear/bin"
install -m 755 "$SRCDIR/bear-driver" "$DESTDIR$PREFIX/libexec/bear/bin/bear-driver"
install -m 755 "$SRCDIR/bear-wrapper" "$DESTDIR$PREFIX/libexec/bear/bin/bear-wrapper"

# preload library (Unix only)
if [ "$HAS_PRELOAD" = true ] && [ -f "$SRCDIR/$PRELOAD_NAME" ]; then
    mkdir -p "$DESTDIR$PREFIX/libexec/bear/$INTERCEPT_LIBDIR"
    install -m 644 "$SRCDIR/$PRELOAD_NAME" "$DESTDIR$PREFIX/libexec/bear/$INTERCEPT_LIBDIR/$PRELOAD_NAME"
fi

# bear entry script
mkdir -p "$DESTDIR$PREFIX/bin"
tmp_bear_sh="$(mktemp)"
trap 'rm -f "$tmp_bear_sh"' EXIT
cat > "$tmp_bear_sh" <<ENTRY_SCRIPT
#!/bin/sh
$PREFIX/libexec/bear/bin/bear-driver "\$@"
ENTRY_SCRIPT
install -m 755 "$tmp_bear_sh" "$DESTDIR$PREFIX/bin/bear"
rm -f "$tmp_bear_sh"
trap - EXIT

# man page
if [ -f "$REPO_ROOT/man/bear.1" ]; then
    mkdir -p "$DESTDIR$PREFIX/share/man/man1"
    install -m 644 "$REPO_ROOT/man/bear.1" "$DESTDIR$PREFIX/share/man/man1/bear.1"
fi

# documentation
mkdir -p "$DESTDIR$PREFIX/share/doc/bear"
if [ -f "$REPO_ROOT/README.md" ]; then
    install -m 644 "$REPO_ROOT/README.md" "$DESTDIR$PREFIX/share/doc/bear/README.md"
fi
if [ -f "$REPO_ROOT/COPYING" ]; then
    install -m 644 "$REPO_ROOT/COPYING" "$DESTDIR$PREFIX/share/doc/bear/COPYING"
fi

# shell completions (optional - only installed when generated files are present)
COMPLETIONS_DIR="$SRCDIR/completions"
if [ -d "$COMPLETIONS_DIR" ]; then
    if [ -f "$COMPLETIONS_DIR/bear.bash" ]; then
        mkdir -p "$DESTDIR$PREFIX/share/bash-completion/completions"
        install -m 644 "$COMPLETIONS_DIR/bear.bash" "$DESTDIR$PREFIX/share/bash-completion/completions/bear"
    fi
    if [ -f "$COMPLETIONS_DIR/_bear" ]; then
        mkdir -p "$DESTDIR$PREFIX/share/zsh/site-functions"
        install -m 644 "$COMPLETIONS_DIR/_bear" "$DESTDIR$PREFIX/share/zsh/site-functions/_bear"
    fi
    if [ -f "$COMPLETIONS_DIR/bear.fish" ]; then
        mkdir -p "$DESTDIR$PREFIX/share/fish/vendor_completions.d"
        install -m 644 "$COMPLETIONS_DIR/bear.fish" "$DESTDIR$PREFIX/share/fish/vendor_completions.d/bear.fish"
    fi
    if [ -f "$COMPLETIONS_DIR/bear.elv" ]; then
        mkdir -p "$DESTDIR$PREFIX/share/elvish/lib"
        install -m 644 "$COMPLETIONS_DIR/bear.elv" "$DESTDIR$PREFIX/share/elvish/lib/bear.elv"
    fi
fi

echo "Bear installed to $PREFIX"
