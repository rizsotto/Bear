# SPDX-License-Identifier: GPL-3.0-or-later
#
# The clang-consumer loop (dogfood-clang-consumer), factored into a single POSIX
# sh function. It is the SAME SHAPE as replay-loop.sh's replay_cdb, but instead
# of re-running the recorded compiler it feeds each sampled entry to a real
# clang-tooling consumer and asks "can the tool build the AST from Bear's
# database?". That is the one check that validates the database WORKS in the tool
# it exists for, not merely that it looks structurally right.
#
# It runs INSIDE the build container only (concatenated into run.sh's `podman run
# sh -c` script): the consumer reads the CAPTURED compile_commands.json and needs
# the recorded sources and generated headers, which exist there only before
# teardown. Unlike replay's bad-directory fault (a host-side selftest.sh
# fixture), the clang-consumer fault demo also runs in-container, because the
# host has no clang (see consumer-fault-demo.sh).
#
# consumer_cdb <cdb-compare> <CDB> <count> <build-dir> <summary-out>
#
#   <cdb-compare>  path to the cdb-compare binary to use for `sample`
#   <CDB>          captured compilation database to feed the consumer (its
#                  containing directory is passed to the consumer as `-p`)
#   <count>        max entries to sample
#   <build-dir>    passed to `sample --build-dir` so the sampler prefers TUs
#                  whose flags do not reference build-dir includes (the same
#                  build-dir-aware selection replay uses)
#   <summary-out>  file to write the OK/FAIL/INCONCLUSIVE tally and any rejected
#                  entries to
#
# THE CONSUMER (chosen empirically, not guessed). The targets build with GCC, so
# clang tooling must consume a gcc-built compile_commands.json. The consumer is:
#
#   clang-tidy --checks='-*,bugprone-assert-side-effect' --allow-no-checks \
#       -p <cdb-dir> <file>
#
# clang-tidy reads the gcc-recorded command from the CDB at `-p <cdb-dir>` and
# mangles the driver-incompatible flags itself (it drops GCC-only flags clang
# does not know) - this cross-compiler consumption is exactly what the check
# validates. A single real check (`bugprone-assert-side-effect`, a cheap, always-
# present check) is enabled to FORCE a full front-end parse of the TU; with no
# check enabled clang-tidy short-circuits and never parses, so the gate would be
# vacuous. `--allow-no-checks` keeps the run from erroring out should that check
# name ever disappear from a future clang-tools.
#
# Why clang-tidy and not `clangd --check`: `clangd --check` runs clangd's
# INTERNAL tweak/refactoring self-tests over the TU and reports their failures as
# "N errors" with a non-zero exit even when the AST built perfectly (observed:
# every curl TU "built the AST" yet reported up to 60 such bogus errors). That
# makes clangd's exit code and error count useless as a database-validity signal.
# clang-tidy with no lint check enabled does a PURE parse: the only `error:`
# diagnostics it emits are genuine front-end errors (a header not found from a
# wrong/missing -I, a bad -x, a missing sysroot) - exactly the database defects
# this check exists to catch.
#
# Categorization (keyed on clang-tidy's `error:` diagnostics, tuned on the real
# curl output - 20/20 OK, the stripped-`-I` fault FAILs):
#   OK            zero `error:` diagnostics: clang-tidy parsed the TU and built
#                 the AST from Bear's recorded command. (Warnings are expected on
#                 a gcc TU under clang and are NOT a defect.)
#   FAIL          one or more `error:` diagnostics, INCLUDING a not-found
#                 #include. This is the load-bearing decision and it relies on a
#                 premise this loop is built around: the consumer runs in the
#                 LIVE post-build container (same `podman run` as the build), so
#                 every header the build saw - source-tree AND generated - is
#                 still on disk. A "file not found" here therefore is NOT a
#                 generated-header-gone artifact (that only happens after
#                 teardown); it means Bear's recorded include paths do not
#                 actually let the tool find a header the build found - exactly
#                 the wrong/missing-`-I` defect this check exists to catch. The
#                 fault demo (consumer-fault-demo.sh) confirms this: stripping an
#                 `-I` from a good entry turns its OK into a not-found `error:` =>
#                 FAIL. On the pinned curl build no GOOD entry trips this (the
#                 build-dir-aware sampler avoids generated-header-heavy TUs).
#   INCONCLUSIVE  the source TU is no longer on disk (a build-time input gone -
#                 the one genuine missing-input case, since the TU source, unlike
#                 a header, can be a generated/temporary file). Reserved narrowly
#                 so the not-found-header signal is not diluted into noise.
#
# NOTE on the OK<->FAIL boundary (the thing to scrutinize): because a not-found
# header is FAIL, a future target whose sampled TUs legitimately depend on a
# generated header the build cleans BEFORE this loop runs would false-FAIL. That
# does not happen for curl/zlib (the build leaves its tree intact and the sampler
# prefers source-tree TUs), and it is the honest trade: keeping not-found as
# INCONCLUSIVE would have masked the very wrong-`-I` defect the check is for. If
# such a target appears, narrow the FAIL rule to "not-found header NOT reachable
# from a build-dir include" rather than blanket-INCONCLUSIVE-ing not-found.
#
# Exit status mirrors replay_cdb so run.sh reuses the same gate mapping:
#   0  PASS          no FAIL and at least one OK (non-vacuous: an all-inconclusive
#                    or empty sample validated nothing and must NOT pass)
#   1  FAIL          at least one entry indicts the database
#   2  INCONCLUSIVE  no FAIL but nothing was actually consumed (all inconclusive
#                    or empty sample)
#   3  ERROR         `cdb-compare sample` itself failed (infra)
consumer_cdb() {
    _cc_compare="$1"
    _cc_cdb="$2"
    _cc_count="$3"
    _cc_builddir="$4"
    _cc_out="$5"

    _cc_ok=0
    _cc_fail=0
    _cc_incon=0
    : >"$_cc_out"

    # clang-tidy reads the database from the directory given to `-p`; that is the
    # directory holding the captured compile_commands.json.
    _cc_cdbdir="$(dirname "$_cc_cdb")"

    "$_cc_compare" sample "$_cc_cdb" --count "$_cc_count" --build-dir "$_cc_builddir" \
        >"$_cc_out.lines" 2>>"$_cc_out"
    _cc_sample_rc=$?
    # A non-zero sample is an infra failure (a broken cdb-compare), NOT a vacuous
    # "nothing matched": signal it distinctly (3 => ERROR).
    if [ "$_cc_sample_rc" -ne 0 ]; then
        printf 'ERROR: cdb-compare sample failed (rc=%s)\n' "$_cc_sample_rc" >>"$_cc_out"
        rm -f "$_cc_out.lines"
        return 3
    fi

    while IFS= read -r _cc_line; do
        [ -n "$_cc_line" ] || continue
        eval "set -- $_cc_line"
        # The first field is the recorded `directory`; the rest is the argv. We do
        # not re-run the compiler - we only need the source FILE to hand to
        # clang-tidy, which then reads the recorded command from the CDB itself.
        _cc_dir="$1"
        shift

        # Find the source file in the recorded argv: the argument naming an
        # existing C/C++ TU. Resolve it relative to the recorded directory when it
        # is not absolute, so the path matches the CDB's `file`. The `-o` output
        # is never matched here (it is a .o, not a source extension), so it needs
        # no special-casing on the build systems we target.
        _cc_file=""
        for _cc_arg in "$@"; do
            case "$_cc_arg" in
                *.c|*.cc|*.cpp|*.cxx|*.c++|*.C|*.m|*.mm)
                    case "$_cc_arg" in
                        /*) _cc_cand="$_cc_arg" ;;
                        *)  _cc_cand="$_cc_dir/$_cc_arg" ;;
                    esac
                    if [ -f "$_cc_cand" ]; then
                        _cc_file="$_cc_cand"
                        break
                    fi
                    ;;
            esac
        done

        if [ -z "$_cc_file" ]; then
            # The TU source is no longer on disk: a build-time input gone, not a
            # database defect. Inconclusive.
            _cc_incon=$((_cc_incon + 1))
            printf 'INCONCLUSIVE (source not on disk): %s\n' "$*" >>"$_cc_out"
            continue
        fi

        # Feed the entry to clang-tidy. A single real check forces a full parse;
        # `-p <cdb-dir>` makes it read Bear's recorded command for this file.
        # `-Wno-error` undoes a recorded `-Werror`: the target is only warning-free
        # under GCC, so a clang warning promoted to an error says nothing about
        # the database (seen on the kernel, built with CONFIG_WERROR=y).
        _cc_diag="$(clang-tidy --checks='-*,bugprone-assert-side-effect' \
            --allow-no-checks --extra-arg=-Wno-error \
            -p "$_cc_cdbdir" "$_cc_file" 2>&1)"

        # The verdict is the presence of front-end `error:` diagnostics. None =>
        # the AST built (warnings are expected on a gcc TU and are ignored). Any
        # error - including a not-found #include, which in the live post-build
        # container is a wrong/missing-`-I` defect, not a generated-header artifact
        # - indicts the database (FAIL). The only missing-INPUT inconclusive is the
        # TU source itself being gone, handled above before the consumer runs.
        # Match only the diagnostic SEVERITY field ': error:' (clang prints
        # `<path>:<line>:<col>: error: ...`, the severity always preceded by
        # ': '), so a warning whose message BODY merely contains the word
        # "error:" cannot false-FAIL a healthy entry.
        if printf '%s\n' "$_cc_diag" | grep -q ': error:'; then
            _cc_fail=$((_cc_fail + 1))
            printf 'FAIL (clang-tidy rejected the entry): %s\n' "$_cc_file" >>"$_cc_out"
            printf '%s\n' "$_cc_diag" | grep ': error:' | head -3 | sed 's/^/  diag: /' >>"$_cc_out"
        else
            _cc_ok=$((_cc_ok + 1))
        fi
    done <"$_cc_out.lines"
    rm -f "$_cc_out.lines"

    printf 'consumer: ok=%s fail=%s inconclusive=%s\n' \
        "$_cc_ok" "$_cc_fail" "$_cc_incon" >>"$_cc_out"

    if [ "$_cc_fail" -gt 0 ]; then
        return 1
    fi
    if [ "$_cc_ok" -eq 0 ]; then
        # Nothing was actually consumed (all inconclusive, or empty sample).
        return 2
    fi
    return 0
}
