#!/bin/bash
# SPDX-License-Identifier: GPL-3.0-or-later
#
# Install script for Bear.
#
# Environment variables:
#   DESTDIR          — installation prefix (default: /usr/local if root, $HOME/.local otherwise)
#   INTERCEPT_LIBDIR — library directory name (default: lib)
#   SOURCE_DIR       — directory containing build artifacts (default: target/release)
#
# Usage:
#   ./scripts/install.sh              # install with defaults
#   ./scripts/install.sh --uninstall  # remove previously installed files
#
#   DESTDIR=/usr INTERCEPT_LIBDIR=lib64 ./scripts/install.sh

set -euxo pipefail

# --- configuration -----------------------------------------------------------

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

DESTDIR="${DESTDIR:-}"
if [ -z "$DESTDIR" ]; then
    if [ "$(id -u)" -eq 0 ]; then
        DESTDIR="/usr/local"
    else
        DESTDIR="$HOME/.local"
    fi
fi

# Resolve to absolute path
DESTDIR="$(cd "$DESTDIR" 2>/dev/null && pwd || echo "$DESTDIR")"

INTERCEPT_LIBDIR="${INTERCEPT_LIBDIR-lib}"

MANIFEST="$DESTDIR/share/bear/install-manifest.txt"

# --- safety guards ------------------------------------------------------------

refuse_root_destdir() {
    if [ "$DESTDIR" = "/" ]; then
        echo "error: refusing to operate with DESTDIR=/ (would clobber the root filesystem)" >&2
        exit 1
    fi
}

validate_intercept_libdir() {
    # Reject empty or whitespace-only values
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
}

# --- artifact discovery -------------------------------------------------------

# When run from the source repo, artifacts are in target/release/.
# When run from a release archive, artifacts are next to the script.
# Override with SOURCE_DIR to use a custom artifact directory (e.g. target/debug/).
find_source_dir() {
    if [ -n "${SOURCE_DIR:-}" ]; then
        echo "$SOURCE_DIR"
    elif [ -d "$REPO_ROOT/target/release" ]; then
        echo "$REPO_ROOT/target/release"
    elif [ -f "$REPO_ROOT/bin/bear-driver" ] || [ -f "$REPO_ROOT/bin/bear-driver.exe" ]; then
        echo "$REPO_ROOT"
    else
        echo "error: cannot find build artifacts in target/release/ or next to the script" >&2
        exit 1
    fi
}

# --- platform detection -------------------------------------------------------

detect_platform() {
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
        MINGW*|MSYS*|CYGWIN*)
            PRELOAD_NAME=""
            HAS_PRELOAD=false
            ;;
        *)
            PRELOAD_NAME="libexec.so"
            HAS_PRELOAD=true
            ;;
    esac
}

# --- install ------------------------------------------------------------------

do_install() {
    refuse_root_destdir
    validate_intercept_libdir
    detect_platform

    SOURCE_DIR="$(find_source_dir)"

    # Start fresh manifest
    mkdir -p "$DESTDIR/share/bear"
    : > "$MANIFEST"

    # bear-driver and bear-wrapper
    mkdir -p "$DESTDIR/share/bear/bin"
    install -m 755 "$SOURCE_DIR/bear-driver" "$DESTDIR/share/bear/bin/bear-driver"
    echo "$DESTDIR/share/bear/bin/bear-driver" >> "$MANIFEST"
    install -m 755 "$SOURCE_DIR/bear-wrapper" "$DESTDIR/share/bear/bin/bear-wrapper"
    echo "$DESTDIR/share/bear/bin/bear-wrapper" >> "$MANIFEST"

    # preload library (Unix only)
    if [ "$HAS_PRELOAD" = true ] && [ -f "$SOURCE_DIR/$PRELOAD_NAME" ]; then
        mkdir -p "$DESTDIR/share/bear/$INTERCEPT_LIBDIR"
        install -m 644 "$SOURCE_DIR/$PRELOAD_NAME" "$DESTDIR/share/bear/$INTERCEPT_LIBDIR/$PRELOAD_NAME"
        echo "$DESTDIR/share/bear/$INTERCEPT_LIBDIR/$PRELOAD_NAME" >> "$MANIFEST"
    fi

    # bear entry script
    mkdir -p "$DESTDIR/bin"
    cat > "$DESTDIR/bin/bear" <<ENTRY_SCRIPT
#!/bin/sh
$DESTDIR/share/bear/bin/bear-driver "\$@"
ENTRY_SCRIPT
    chmod 755 "$DESTDIR/bin/bear"
    echo "$DESTDIR/bin/bear" >> "$MANIFEST"

    # man page
    if [ -f "$REPO_ROOT/man/bear.1" ]; then
        mkdir -p "$DESTDIR/share/man/man1"
        install -m 644 "$REPO_ROOT/man/bear.1" "$DESTDIR/share/man/man1/bear.1"
        echo "$DESTDIR/share/man/man1/bear.1" >> "$MANIFEST"
    fi

    # documentation
    mkdir -p "$DESTDIR/share/doc/bear"
    if [ -f "$REPO_ROOT/README.md" ]; then
        install -m 644 "$REPO_ROOT/README.md" "$DESTDIR/share/doc/bear/README.md"
        echo "$DESTDIR/share/doc/bear/README.md" >> "$MANIFEST"
    fi
    if [ -f "$REPO_ROOT/COPYING" ]; then
        install -m 644 "$REPO_ROOT/COPYING" "$DESTDIR/share/doc/bear/COPYING"
        echo "$DESTDIR/share/doc/bear/COPYING" >> "$MANIFEST"
    fi

    # record the manifest itself
    echo "$MANIFEST" >> "$MANIFEST"

    echo "Bear installed to $DESTDIR"
    echo "Manifest written to $MANIFEST"
}

# --- uninstall ----------------------------------------------------------------

do_uninstall() {
    refuse_root_destdir
    validate_intercept_libdir

    if [ ! -f "$MANIFEST" ]; then
        echo "error: no install manifest found at $MANIFEST" >&2
        echo "Cannot uninstall without a manifest." >&2
        exit 1
    fi

    # Remove each file listed in the manifest
    while IFS= read -r file; do
        if [ -f "$file" ]; then
            rm -f "$file"
            echo "removed: $file"
        fi
    done < "$MANIFEST"

    # Remove empty directories (deepest first)
    for dir in \
        "$DESTDIR/share/bear/bin" \
        "$DESTDIR/share/bear/$INTERCEPT_LIBDIR" \
        "$DESTDIR/share/bear" \
        "$DESTDIR/share/doc/bear" \
        "$DESTDIR/share/man/man1" \
        "$DESTDIR/share/man" \
        "$DESTDIR/share/doc" \
        "$DESTDIR/share" \
        "$DESTDIR/bin" \
    ; do
        rmdir "$dir" 2>/dev/null || true
    done

    echo "Bear uninstalled from $DESTDIR"
}

# --- main ---------------------------------------------------------------------

case "${1:-}" in
    --uninstall)
        do_uninstall
        ;;
    "")
        do_install
        ;;
    *)
        echo "usage: $0 [--uninstall]" >&2
        exit 1
        ;;
esac
