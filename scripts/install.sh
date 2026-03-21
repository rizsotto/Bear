#!/bin/bash
# SPDX-License-Identifier: GPL-3.0-or-later
#
# Install script for Bear.
#
# Environment variables:
#   PREFIX           — installation prefix (default: /usr/local if root, $HOME/.local otherwise)
#   INTERCEPT_LIBDIR — library directory name (default: lib)
#   SRCDIR           — directory containing build artifacts (default: target/release)
#
# Usage:
#   ./scripts/install.sh              # install with defaults
#   ./scripts/install.sh --uninstall  # remove previously installed files
#
#   PREFIX=/usr INTERCEPT_LIBDIR=lib64 ./scripts/install.sh

set -euxo pipefail

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

UNINSTALL_SCRIPT="$PREFIX/share/bear/uninstall.sh"

# --- safety guards ------------------------------------------------------------

refuse_root_prefix() {
    if [ "$PREFIX" = "/" ]; then
        echo "error: refusing to operate with PREFIX=/ (would clobber the root filesystem)" >&2
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
# Override with SRCDIR to use a custom artifact directory (e.g. target/debug/).
find_source_dir() {
    if [ -n "${SRCDIR:-}" ]; then
        echo "$SRCDIR"
    elif [ -d "$REPO_ROOT/target/release" ]; then
        echo "$REPO_ROOT/target/release"
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
        *)
            PRELOAD_NAME=""
            HAS_PRELOAD=false
            ;;
    esac
}

# --- install ------------------------------------------------------------------

do_install() {
    refuse_root_prefix
    validate_intercept_libdir
    detect_platform

    SRCDIR="$(find_source_dir)"

    # Start generating uninstall script (create directory first)
    mkdir -p "$PREFIX/share/bear"
    cat > "$UNINSTALL_SCRIPT" <<'UNINSTALL_HEADER'
#!/bin/sh
# Bear uninstall script
# This script was generated during installation and removes all installed files.
# Usage: sh uninstall.sh

set -e

UNINSTALL_HEADER

    # Helper to emit directory removal (only if not a protected system directory)
    emit_rmdir() {
        local dir="$1"
        local boundary="$2"
        local current="$dir"

        while [ "$current" != "$boundary" ] && [ -n "$current" ] && [ "$current" != "/" ]; do
            # Don't emit rmdir for common system directories
            case "$current" in
                /usr|/usr/local|/opt|/etc)
                    # Protected - don't try to remove
                    ;;
                *)
                    echo "rmdir '$current' 2>/dev/null || true" >> "$UNINSTALL_SCRIPT"
                    ;;
            esac
            current="$(dirname "$current")"
        done
    }

    # bear-driver and bear-wrapper
    echo "# Remove bear binaries" >> "$UNINSTALL_SCRIPT"
    mkdir -p "$PREFIX/share/bear/bin"
    install -m 755 "$SRCDIR/bear-driver" "$PREFIX/share/bear/bin/bear-driver"
    echo "rm -f '$PREFIX/share/bear/bin/bear-driver'" >> "$UNINSTALL_SCRIPT"
    install -m 755 "$SRCDIR/bear-wrapper" "$PREFIX/share/bear/bin/bear-wrapper"
    echo "rm -f '$PREFIX/share/bear/bin/bear-wrapper'" >> "$UNINSTALL_SCRIPT"
    emit_rmdir "$PREFIX/share/bear/bin" "$PREFIX"

    # preload library (Unix only)
    if [ "$HAS_PRELOAD" = true ] && [ -f "$SRCDIR/$PRELOAD_NAME" ]; then
        echo "" >> "$UNINSTALL_SCRIPT"
        echo "# Remove preload library" >> "$UNINSTALL_SCRIPT"
        mkdir -p "$PREFIX/share/bear/$INTERCEPT_LIBDIR"
        install -m 644 "$SRCDIR/$PRELOAD_NAME" "$PREFIX/share/bear/$INTERCEPT_LIBDIR/$PRELOAD_NAME"
        echo "rm -f '$PREFIX/share/bear/$INTERCEPT_LIBDIR/$PRELOAD_NAME'" >> "$UNINSTALL_SCRIPT"
        emit_rmdir "$PREFIX/share/bear/$INTERCEPT_LIBDIR" "$PREFIX"
    fi

    # bear entry script
    echo "" >> "$UNINSTALL_SCRIPT"
    echo "# Remove bear entry script" >> "$UNINSTALL_SCRIPT"
    mkdir -p "$PREFIX/bin"
    tmp_bear_sh="$(mktemp)"
    trap 'rm -f "$tmp_bear_sh"' EXIT
    cat > "$tmp_bear_sh" <<ENTRY_SCRIPT
#!/bin/sh
$PREFIX/share/bear/bin/bear-driver "\$@"
ENTRY_SCRIPT
    install -m 755 "$tmp_bear_sh" "$PREFIX/bin/bear"
    rm -f "$tmp_bear_sh"
    trap - EXIT
    echo "rm -f '$PREFIX/bin/bear'" >> "$UNINSTALL_SCRIPT"
    emit_rmdir "$PREFIX/bin" "$PREFIX"

    # man page
    if [ -f "$REPO_ROOT/man/bear.1" ]; then
        echo "" >> "$UNINSTALL_SCRIPT"
        echo "# Remove man page" >> "$UNINSTALL_SCRIPT"
        mkdir -p "$PREFIX/share/man/man1"
        install -m 644 "$REPO_ROOT/man/bear.1" "$PREFIX/share/man/man1/bear.1"
        echo "rm -f '$PREFIX/share/man/man1/bear.1'" >> "$UNINSTALL_SCRIPT"
        emit_rmdir "$PREFIX/share/man/man1" "$PREFIX"
    fi

    # documentation
    echo "" >> "$UNINSTALL_SCRIPT"
    echo "# Remove documentation" >> "$UNINSTALL_SCRIPT"
    mkdir -p "$PREFIX/share/doc/bear"
    if [ -f "$REPO_ROOT/README.md" ]; then
        install -m 644 "$REPO_ROOT/README.md" "$PREFIX/share/doc/bear/README.md"
        echo "rm -f '$PREFIX/share/doc/bear/README.md'" >> "$UNINSTALL_SCRIPT"
    fi
    if [ -f "$REPO_ROOT/COPYING" ]; then
        install -m 644 "$REPO_ROOT/COPYING" "$PREFIX/share/doc/bear/COPYING"
        echo "rm -f '$PREFIX/share/doc/bear/COPYING'" >> "$UNINSTALL_SCRIPT"
    fi
    emit_rmdir "$PREFIX/share/doc/bear" "$PREFIX"

    # shell completions (optional — only installed when generated files are present)
    COMPLETIONS_DIR="$SRCDIR/completions"
    if [ -d "$COMPLETIONS_DIR" ]; then
        echo "" >> "$UNINSTALL_SCRIPT"
        echo "# Remove shell completions" >> "$UNINSTALL_SCRIPT"

        if [ -f "$COMPLETIONS_DIR/bear.bash" ]; then
            mkdir -p "$PREFIX/share/bash-completion/completions"
            install -m 644 "$COMPLETIONS_DIR/bear.bash" "$PREFIX/share/bash-completion/completions/bear"
            echo "rm -f '$PREFIX/share/bash-completion/completions/bear'" >> "$UNINSTALL_SCRIPT"
            emit_rmdir "$PREFIX/share/bash-completion/completions" "$PREFIX"
        fi
        if [ -f "$COMPLETIONS_DIR/_bear" ]; then
            mkdir -p "$PREFIX/share/zsh/site-functions"
            install -m 644 "$COMPLETIONS_DIR/_bear" "$PREFIX/share/zsh/site-functions/_bear"
            echo "rm -f '$PREFIX/share/zsh/site-functions/_bear'" >> "$UNINSTALL_SCRIPT"
            emit_rmdir "$PREFIX/share/zsh/site-functions" "$PREFIX"
        fi
        if [ -f "$COMPLETIONS_DIR/bear.fish" ]; then
            mkdir -p "$PREFIX/share/fish/vendor_completions.d"
            install -m 644 "$COMPLETIONS_DIR/bear.fish" "$PREFIX/share/fish/vendor_completions.d/bear.fish"
            echo "rm -f '$PREFIX/share/fish/vendor_completions.d/bear.fish'" >> "$UNINSTALL_SCRIPT"
            emit_rmdir "$PREFIX/share/fish/vendor_completions.d" "$PREFIX"
        fi
        if [ -f "$COMPLETIONS_DIR/bear.elv" ]; then
            mkdir -p "$PREFIX/share/elvish/lib"
            install -m 644 "$COMPLETIONS_DIR/bear.elv" "$PREFIX/share/elvish/lib/bear.elv"
            echo "rm -f '$PREFIX/share/elvish/lib/bear.elv'" >> "$UNINSTALL_SCRIPT"
            emit_rmdir "$PREFIX/share/elvish/lib" "$PREFIX"
        fi
    fi

    # Remove the uninstall script itself
    echo "" >> "$UNINSTALL_SCRIPT"
    echo "# Remove uninstall script" >> "$UNINSTALL_SCRIPT"
    echo "rm -f '$UNINSTALL_SCRIPT'" >> "$UNINSTALL_SCRIPT"
    emit_rmdir "$PREFIX/share/bear" "$PREFIX"

    echo "" >> "$UNINSTALL_SCRIPT"
    echo "echo 'Bear uninstalled from $PREFIX'" >> "$UNINSTALL_SCRIPT"

    # Make uninstall script non-executable (must be invoked explicitly)
    chmod 644 "$UNINSTALL_SCRIPT"

    echo "Bear installed to $PREFIX"
    echo "Uninstall script written to $UNINSTALL_SCRIPT"
}

# --- uninstall ----------------------------------------------------------------

do_uninstall() {
    refuse_root_prefix

    if [ ! -f "$UNINSTALL_SCRIPT" ]; then
        echo "error: no uninstall script found at $UNINSTALL_SCRIPT" >&2
        echo "Cannot uninstall without the uninstall script." >&2
        exit 1
    fi

    # Execute the generated uninstall script
    sh "$UNINSTALL_SCRIPT"
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
