# Dogfooding harness specification

This is the maintainer's reference for the harness under `tests/dogfooding/`:
the `dogfood-*` contracts it satisfies, the rationale behind each check, and how
to extend it. For running the harness, see `README.md`.

These are contracts on the TEST HARNESS, not on Bear, so they live here and NOT
under `docs/requirements/` (which is reserved for Bear's own contracts).

Scope: non-automated, run by the maintainer at release time. Bear's installed
release binaries are run against a real project inside a throwaway container, and
the captured compilation database is validated. Sources and toolchain live only
in the container, never in the repo or the devcontainer image (the
host-orchestrated Podman model: the orchestrator is POSIX `sh` on the host,
each target runs in a per-project throwaway container).

## How the contracts are organized

- **Run model** -- how a run is built and how outcomes are classified:
  `dogfood-run-containerized`, `dogfood-fixed-paths`, `dogfood-pinned-target`,
  `dogfood-preflight`, `dogfood-build-failure-taxonomy`.
- **Per-target default validation** -- chosen by a `VALIDATION` selector in the
  target's `config.env`: `golden` (zlib) or `oracle` (curl). `none` (ffmpeg,
  kernel) means no default gate; the target is run only with an on-demand check.
- **On-demand checks** -- selected by flag, need no maintained baseline, run
  against any target: `dogfood-determinism`, `dogfood-invariants`,
  `dogfood-replay`, `dogfood-clang-consumer`. Each builds+captures once (twice
  for determinism) and skips the golden/oracle gate.
- **Fault demonstrations** -- each check must demonstrably catch a fault:
  `dogfood-injected-fault` (host-side `selftest.sh`) and the in-container
  consumer fault demo.
- **Metrics** -- `dogfood-metrics-collect`, an additive profiling modifier.

All comparison, normalization, and gating is done by `cdb-compare` (package
`bear-test-tools`), so the harness needs no `jq`. `run.sh` runs the copy that
ships inside the target image (dogfood-run-containerized), so it needs no host
binary and there is no glibc-mismatch risk from copying the image's dynamically
linked comparator onto an arbitrary host. The one exception is `selftest.sh`,
which never builds an image and so runs a locally built
`target/release/cdb-compare` on the host -- built and run in the same
environment, so no cross-libc concern.

## dogfood-run-containerized

The suite builds a real project inside a throwaway container using the
installed, release Bear binaries, leaving the repo working tree and the
devcontainer image unmodified. A release/install build is required (not a debug
tree) so `libexec.so` sits at the path the binary resolves via
`INTERCEPT_LIBDIR`; a mismatch makes Bear run but capture nothing, so the
harness verifies interception actually occurred (non-empty capture) before
trusting a run.

Implementation: Bear is built inside the base image from `git archive HEAD`
(committed files only) and installed with `scripts/install.sh` to `PREFIX=/opt/bear`
(default `INTERCEPT_LIBDIR=lib`), giving the layout
`/opt/bear/bin/bear` -> `/opt/bear/libexec/bear/bin/bear-driver` +
`../lib/libexec.so`. The base carries no Rust toolchain (multi-stage build).

The comparator stays in the container model too. The base image also installs
`/opt/bear/bin/cdb-compare`, and every host-side gate (golden, oracle,
determinism, invariants, rebless) runs it there rather than on the host, via the
`cdb_compare` helper in `lib.sh`: it `podman run`s the target image with the run
directory bind-mounted read-only at `/run`, passing `/run/<file>` paths, and the
report/exit code propagate back to the host. So `run.sh` needs no host
`cdb-compare` (removing the copy-out fragility of a dynamically linked binary),
and the comparison is the same version that shipped with the Bear under test.
The golden compare stages the committed golden into the run directory first, so
one read-only mount covers both sides; rebless captures `normalize`'s stdout on
the host and installs it atomically. The in-container `sample` for replay/consumer
already used this image binary.

## dogfood-fixed-paths

The build runs at fixed, well-known container paths so absolute paths in
`directory` (and in arguments) do not vary between runs, making command lines
reproducible and letting the golden check pass with little or no normalization.

Implementation: zlib builds in-tree at the fixed path `/src` (its `./configure`
has no usable out-of-tree mode). The config pins `SRC_DIR=/src`.

## dogfood-pinned-target

A dogfooding configuration is (base image digest, target source revision,
target build type); all are pinned so the golden stays stable. An unpinned base
would let a package update change the compiler under a fixed golden; an unpinned
build type would change the recorded flags. Release is the default build type.

Implementation: `targets/zlib/config.env` pins the fedora:44 base by digest, the
zlib 1.3.1 tarball by URL + sha256, and `BUILD_TYPE=Release`
(`CFLAGS="-O3 -DNDEBUG"`).

## dogfood-preflight

Before launching any container, the suite verifies free disk against a
per-target minimum, that the pinned image digest is present/pullable, and that
the image carries a working Bear (the non-empty-capture check). It exits
non-zero with a clear diagnostic on any failure, so a run never starts only to
leave a torn scratch behind.

Implementation: `run.sh` step 1 checks free disk on the podman graphroot
(`podman info` + `df -Pk`) against `MIN_FREE_KIB` and resolves/pulls the pinned
digest; step 4 runs a trivial `bear -- gcc -c` and asserts the capture is
non-empty before the real build.

## dogfood-build-failure-taxonomy

The harness distinguishes a target build failure (network, source fetch, OOM,
missing target dep, configure/make error) from a Bear/harness failure, and
reports them as distinct outcomes. A target build that fails for its own reasons
is "inconclusive", not a Bear regression.

Implementation: four outcomes with distinct exit codes -
`PASS=0`, `FAIL=1` (golden regression, or oracle matched-TU divergence),
`INCONCLUSIVE=2` (target build failed for its own reasons: target image build,
configure/make), `ERROR=3` (harness or Bear-infra: podman missing, disk/digest
preflight, base or target image build, empty capture, an oracle that matched 0
TUs). `run.sh` prints one final `OUTCOME:` status line.

## dogfood-golden-regression

For the pinned target, the suite compares Bear's output against a tracked golden
using `cdb-compare compare` (order-independent multiset equivalence) and flags
any change. The golden is a frozen, full normalized compilation database emitted
with `cdb-compare normalize --sort`, committed under `tests/dogfooding/goldens/`
(NOT under the git-ignored `results/`). The comparison binary is reused as-is; no
hash-manifest tooling is added. The golden is a change-detector, not an
independent proof of correctness.

Implementation: `goldens/zlib/compile_commands.json` is the frozen golden; the
gate is `cdb-compare compare <golden> <fresh>` run in the target image (via
`cdb_compare`, dogfood-run-containerized), with the golden staged into the run
directory so one read-only mount covers both sides. No normalization flags are
used (same pinned image + fixed `/src` => raw multiset matches); if a benign
compiler-path diff ever appears, add `--substitute-compiler cc` to both the
bless and the check, consistently.

## dogfood-golden-rebless

There is a documented, deliberate procedure to regenerate the golden when a
behavior change is intentional, so re-blessing is explicit and reviewable rather
than automatic.

Implementation: `run.sh --rebless <target>` runs the full pipeline, then instead
of gating runs `cdb-compare normalize --sort <fresh>` in the target image (via
`cdb_compare`), captures its stdout on the host, and installs it atomically at
the golden path, reporting "reblessed" for the maintainer to review and commit.
See README.md for the maintainer-facing procedure.

## dogfood-oracle-cmake

For a CMake-native target, the suite compares Bear's output against the database
CMake itself emits (`CMAKE_EXPORT_COMPILE_COMMANDS=ON`), scoped to the
intersection of translation units. The check passes when matched TUs have
equivalent flags under normalization. Entries present in only one database are
logged as "extras", never failures: CMake lists configured TUs with
configure-time flags, while Bear records the actual make-time command, so a
whole-database equality would be pure noise. The oracle renews itself when the
target updates; no hand-maintained baseline. The oracle target is curl (CMake-
native, small); zlib stays the golden target (autotools).

Implementation: `targets/curl/config.env` sets `VALIDATION=oracle` and pins the
fedora:44 base by digest, curl 8.11.1 by URL + sha256, and `BUILD_TYPE=Release`.
The in-container command (dogfood-fixed-paths) configures out-of-tree (source
`/src`, build `/build`) with `-DCMAKE_EXPORT_COMPILE_COMMANDS=ON` and the
optional dependencies turned off (no TLS/nghttp2/zlib/zstd/brotli/psl/idn/ssh/
ldap), so curl builds with just cmake+gcc+make. The CMake *configure* step is
NOT wrapped by Bear (it is not a compile); only `cmake --build` is, so Bear's
capture lands at `/out/compile_commands.json` and CMake's reference database is
copied to `/out/oracle.json`. Both are pulled out with `podman cp`.

The comparison is done entirely by the image's `cdb-compare` (via `cdb_compare`)
- no allow-list file, no shell JSON surgery:

```sh
cdb-compare compare --intersection --substitute-compiler cc \
    --output-from-o --drop-dependency-flags  <bear.json> <cmake.json>
```

`tests/tools/SPEC.md` documents what each flag does; the reason each is needed
*here* is:

- `--output-from-o` - Bear encodes `output` relative to `directory` and CMake
  relative to the build root, so TUs must match by source plus the absolute
  object path rather than the raw `output` string; it also keeps a source
  compiled into several targets (shared lib, tool) distinct.
- `--drop-dependency-flags` - the make-time compile carries the `-M*` depfile
  flags and CMake's configure-time export omits them (see
  dogfood-divergence-report); they touch only the `.d` side-file.
- `--substitute-compiler cc` - absorbs any compiler-driver path difference
  between the two producers.
- `--intersection` - gates on the `differing` set alone, and its non-vacuity
  clause turns a comparison that matched 0 TUs into a failure rather than a green
  no-op.

Exit 0 = matched TUs equivalent (PASS); exit 1 = a real matched-TU divergence
(FAIL).

## dogfood-divergence-report

The Bear-only and CMake-only "extras" are reported for review, never a failure.
On the pinned build there are 0 Bear-only and ~156 CMake-only extras (CMake
lists every configured TU, including ones a given build target does not actually
compile).

The one known-benign argument difference on matched TUs - the dependency-file
generation flags (`-MD`/`-MMD`/`-MP` and the argument-consuming
`-MF`/`-MT`/`-MQ`/`-MJ`) - is removed by `cdb-compare --drop-dependency-flags`
before the gate. These flags only control the build system's `.d` side-file,
never the object or how a Clang tool parses the TU, so dropping them is a tested,
documented operation of the comparator rather than a hand-maintained shell
allow-list. On the pinned curl build this group is the ENTIRE matched-but-
differing set (221/221 TUs); with it dropped, `differing` is empty and the gate
passes. `run.sh` writes the comparator's full report (its extras lists plus a
`summary:` line) to `results/<target>/<label>/oracle-report.txt`, with a
machine-readable `oracle-compare.json` alongside. There is no rebless for oracle
targets - the oracle is self-renewing, so `--rebless` is rejected for them.

If a future oracle target exhibits a different benign argument difference that
`--drop-dependency-flags` does not cover, extend the comparator with a tested
rule rather than reintroducing a shell allow-list - the comparison logic stays
in one unit-tested place (see "Extending cdb-compare" below).

## dogfood-determinism

The suite can run the same target twice and assert Bear's two outputs are
equivalent under `cdb-compare`. Any difference indicates non-determinism or a
race in Bear itself; the build is its own reference, no golden and no oracle
required. This is target-agnostic: it runs for any target, golden or oracle.

Implementation: `run.sh --determinism <target>` runs the shared preflight, base
+ target image builds, and the non-empty-capture smoke (exactly as a normal
run), then runs the target build TWICE in two distinct fresh throwaway
containers off the same pinned image (`build_and_capture` in `lib.sh`, invoked
once per container - the same body the normal single-run path uses, so the
build-failure taxonomy is identical), and compares the two captures with
`cdb-compare compare <run1> <run2>` (via `cdb_compare`, in the target image). NO
normalization flags are used: the fixed
build paths (`/src`, `/build`) make the two captures multiset-equivalent at the
source, and `cdb-compare` is order-independent, so build parallelism does not
matter. The golden/oracle gate is SKIPPED entirely (determinism is its own
check). `--determinism` with `--rebless` is rejected (ERROR: no golden is
involved); `--inject-fault` outside `--determinism` is likewise an ERROR.

Outcome (reusing the existing taxonomy): both captures equivalent => PASS;
captures differ => FAIL (real Bear non-determinism / a race), with the
`cdb-compare compare` diff saved to `results/<target>/<label>/determinism-diff.txt`
(and `.json`); either build failing for its own reasons => INCONCLUSIVE;
podman/infra/empty-capture => ERROR. Both captures
are written as `compile_commands.run1.json` and `compile_commands.run2.json`.
Determinism is verified on both zlib and curl.

Self-test (the check must demonstrably catch a fault):
`run.sh --determinism --inject-fault <target>` perturbs the SECOND build with an
extra compiler flag so the two builds legitimately diverge, and the check is
shown to FAIL. The fault is injected as a real, different second build, NOT by
editing captured JSON by hand: `run.sh` passes a non-empty `INJECT_CFLAGS` (an
extra `-D...` macro) into the second container only, and each target's
`config.env` threads `${INJECT_CFLAGS:-}` into that build's compiler flags
(`CFLAGS` for zlib's configure, `CMAKE_C_FLAGS` for curl's cmake). On a normal
run and on determinism run 1 the value is empty, a no-op. The FAIL run's
`determinism-diff.txt` shows the injected flag present in run 2's arguments and
absent in run 1's. Both directions are verified for zlib and curl.

Scope boundary: the DROPPED, DUPLICATED, and CORRUPTED-ENTRY faults are NOT
determinism's territory - they belong to `dogfood-invariants`
(dropped/duplicated/empty-arguments) and `dogfood-replay` (corrupted
`directory`), both specified below. Determinism's own fault model is a divergent
second build, which is exactly what `--inject-fault` exercises.

## dogfood-invariants

For any target, the suite asserts structural invariants on Bear's single
capture: every entry has non-empty `arguments`, every entry has a non-empty
`directory` (a blank `directory` is non-replayable - a `cd ""` consumer lands in
the wrong tree), there are no TRUE duplicates (identical `file` + `output` +
normalized `arguments` - a source compiled into different outputs with different
flags is legitimate, not a duplicate), and the entry count is within a configured
tolerance of the number of object files the build produced. Each invariant is
independently reported. No golden, no oracle, no maintained baseline.

Implementation: `run.sh --invariants <target>` runs the shared preflight, image
builds, and smoke, builds+captures once (`build_and_capture`), then gates on the
exit code of one in-image `cdb-compare invariants --drop-dependency-flags
--expected-objects <N> --tolerance <PCT> --format human <capture>` (via
`cdb_compare`). The whole
structural check - non-empty-arguments, non-empty-directory, no-true-duplicates,
and the entry-count band - lives in the unit-tested comparator (defined in
`tests/tools/SPEC.md`); the harness only supplies `<N>` and `<PCT>` and gates
the exit code (no JSON parsed in shell).
Exit 0 => PASS; exit 1 => FAIL (Bear produced a malformed CDB); a non-1 error or
a build/infra failure maps to ERROR/INCONCLUSIVE via `build_and_capture`. The
human report is saved to `results/<target>/<label>/invariants-report.txt`.

The object count `<N>` is taken IN the container before teardown, by a
per-target `OBJECT_COUNT_CMD` (config.env) written to `/out/object_count` and
pulled out. The instrument is per-target because "objects produced" is not
always "*.o files still on disk":

- curl / ffmpeg leave every object on disk, so the default
  `find $OBJECTS_DIR -name '*.o' | wc -l` is exact;
- zlib's in-tree `make` compiles each source twice (static + PIC) and DELETES
  the PIC objects at link time (19 survive, 34 produced), so it counts make's
  OWN dependency graph (`make -Bn`);
- the kernel aggregates objects into built-in.a / linked .o (a `find`
  overcounts) AND a `make -Bn` dry-run does not enumerate its recursive compiles
  (it counts ~0), so it counts Kbuild's per-compile `.foo.o.cmd` files instead.

All are build-system-native, cleanup-independent, Bear-independent. `<PCT>` is
the per-target `OBJECT_TOLERANCE_PCT`.

Graceful degradation: the entry-count cross-check is opt-in and must never block
the always-on structural checks. If `OBJECT_COUNT_CMD` cannot produce a number
(0 / empty), `run.sh` warns and runs `cdb-compare invariants` WITHOUT
`--expected-objects` (entry-count skipped), still gating on non-empty-arguments,
non-empty-directory, and no-true-duplicates. A target whose independent object
count is genuinely hard still gets the structural invariants.

Scale: validated on two larger gate-less (`VALIDATION=none`) targets - ffmpeg
(~1945 TUs, exact: entries == objects == 1945) and the Linux kernel (x86_64
defconfig, ~3000 TUs: Kbuild `.cmd` count 2991 vs entries 3004, a 0.4% match).
The entry-count band is thus a real coverage cross-check at scale (did Bear
capture ~every compile?), not just a small-target nicety; entries == objects
exactly on zlib (34) and curl (221). The kernel run also proves Bear stays
deterministic and its process-tree teardown holds under a high-`-j` build.

## dogfood-replay

The suite takes a sample of Bear's entries and replays each recorded command in
its recorded `directory` with `-fsyntax-only` appended, asserting the compiler
accepts the recorded arguments. A command that fails to replay indicates a
malformed entry (wrong cwd, missing or mangled flag). Sampling size is
configurable. In a throwaway container a generated header may no longer exist,
so a replay failure for a TU that depends on a missing input is *inconclusive*,
not a Bear failure; the sampler preferentially selects TUs whose flags do not
reference build-dir includes to keep replay meaningful.

Implementation: `run.sh --replay[=N] <target>` (default N=20; also `--replay N`)
runs the shared pipeline, builds+captures once, and replays INSIDE the build
container as part of the same `podman run` - the recorded sources and generated
headers exist there only before teardown. The replay loop is one POSIX function
(`replay-loop.sh`) read into the in-container script; the in-image
`/opt/bear/bin/cdb-compare sample <capture> --count N --build-dir <BUILD_DIR>`
selects up to N replayable entries (build-dir-aware ranking is defined in
`tests/tools/SPEC.md`) and emits one shell-quoted replay line per entry
(`directory` then argv). Each is replayed as
`( cd "$dir" && "$@" -fsyntax-only )` and tallied:

- **OK** - the compiler accepted the recorded arguments.
- **INCONCLUSIVE** - the failure stderr matches a missing-file diagnostic ("No
  such file" / "file not found" / "not found") - a build-time input gone after
  teardown.
- **FAIL** - any other failure, including a `directory` that does not exist (a
  corrupted-directory fault, caught before the compile by a `-d` test).

The function writes its tally and any failing commands to `/out/replay_result`
and its return code to `/out/replay_rc`, both pulled out and gated on without
parsing JSON. `BUILD_DIR` comes from config.env (curl `/build`; zlib has no
out-of-tree dir, so its `BUILD_DIR` is `/src`).

Gate (non-vacuity, mirroring the oracle): any real FAIL => FAIL (a malformed
entry); all OK with inconclusive allowed => PASS; EVERY sampled entry
inconclusive (nothing actually verified) => INCONCLUSIVE (replay must not pass
vacuously). On the pinned builds all 20 sampled entries replay OK for both zlib
and curl (0 FAIL, 0 INCONCLUSIVE).

## dogfood-clang-consumer

Feed a sample of the captured database to a REAL clang-tooling consumer over the
recorded entries, IN-container, and assert the tool accepts each. This is the
only check that validates the database WORKS in the tool it exists for -
catching a structurally-valid but semantically-broken DB (a wrong/missing `-I`,
a dropped `-x`, a missing sysroot) that the structural invariants and the replay
check both miss. Same shape as dogfood-replay: reuse `cdb-compare sample`,
categorize OK / FAIL / INCONCLUSIVE, non-vacuous (an all-inconclusive or empty
sample is INCONCLUSIVE, never a vacuous PASS).

Implementation: `run.sh --consumer[=N] <target>` (default N=20; also `--consumer
N`) runs the shared pipeline, builds+captures once, and runs the consumer INSIDE
the build container as part of the same `podman run` - the recorded sources and
generated headers exist there only before teardown, AND the clang tooling lives
in the image (the host has no clang). The consumer loop is one POSIX function
(`consumer-loop.sh`) read into the in-container script, mirroring how
`replay-loop.sh` is wired. It is additive/standalone like `--replay`: mutually
exclusive with the other check modes, allowed on `VALIDATION=none` targets, and
gated on an in-container return code mapped to PASS/FAIL/INCONCLUSIVE/ERROR.

The consumer (chosen empirically, not guessed - the targets build with GCC, so
clang tooling must consume a gcc-built database) is:

```sh
clang-tidy --checks='-*,bugprone-assert-side-effect' --allow-no-checks \
    -p <cdb-dir> <file>
```

clang-tidy reads the gcc-recorded command from the CDB at `-p <cdb-dir>` and
mangles the driver-incompatible flags itself (dropping GCC-only flags clang does
not know) - that cross-compiler consumption is precisely what the check
validates. A single real check forces a full front-end parse (with NO check
enabled clang-tidy short-circuits and never parses, so the gate would be
vacuous); `--allow-no-checks` keeps the run from erroring should that check name
disappear from a future clang-tools. `clangd --check` was rejected as the
consumer: it runs clangd's INTERNAL tweak/refactoring self-tests over the TU and
reports their failures as "N errors" with a non-zero exit even when the AST built
perfectly (every curl TU "built the AST" yet reported up to 60 such bogus
errors), making its exit code useless as a database-validity signal. clang-tidy
with no lint check does a pure parse, so the only `error:` diagnostics are
genuine front-end errors. The consumer also passes `--extra-arg=-Wno-error`: a
recorded `-Werror` (the kernel's `CONFIG_WERROR=y`) would promote clang-only
warnings on a gcc-clean TU to errors, which says nothing about the database.

Categorization (keyed on clang-tidy's `error:` diagnostics):

- **OK** - zero `error:` diagnostics: clang-tidy parsed the TU and built the AST
  from Bear's recorded command. Warnings are expected on a gcc TU under clang and
  are NOT a defect.
- **FAIL** - one or more `error:` diagnostics, INCLUDING a not-found `#include`.
  This relies on the loop's premise: the consumer runs in the LIVE post-build
  container, so every header the build saw (source-tree AND generated) is still
  on disk. A "file not found" therefore is NOT a generated-header-gone artifact
  (that only happens after teardown); it means Bear's recorded include paths do
  not let the tool find a header the build found - the wrong/missing-`-I` defect
  the check exists for.
- **INCONCLUSIVE** - the TU source itself is no longer on disk (the one genuine
  missing-input case, since a TU source, unlike a header, can be a generated
  file). Kept narrow so the not-found-header FAIL signal is not diluted.

Gate (mirroring the oracle / replay non-vacuity): any FAIL => FAIL; all OK
(inconclusive allowed) => PASS; EVERY sampled entry inconclusive (nothing
consumed) => INCONCLUSIVE. The tally and any rejected entries are written to
`results/<target>/<label>/consumer_result`, the return code to `consumer_rc`,
both pulled out and gated on without parsing JSON.

Signal quality on a gcc-built target: meaningful, not noise-dominated. On the
pinned curl build all 20 sampled entries come back OK (`ok=20 fail=0
inconclusive=0`): clang-tidy consumes the gcc-recorded database cleanly, parses
every TU without error, and only the deliberate fault flips a verdict. The
build-dir-aware sampler (`--build-dir <BUILD_DIR>`) avoids generated-header-heavy
TUs, which keeps good entries from tripping the not-found FAIL.

Boundary to scrutinize (the OK<->FAIL line): because a not-found header is FAIL,
a future target whose sampled TUs legitimately depend on a generated header the
build CLEANS before this loop runs would false-FAIL. That does not happen for
curl/zlib (the build leaves its tree intact and the sampler prefers source-tree
TUs); it is the honest trade - keeping not-found as INCONCLUSIVE would have
masked the very wrong-`-I` defect the check is for. If such a target appears,
narrow the FAIL rule to "not-found header NOT reachable from a build-dir
include", rather than blanket-INCONCLUSIVE-ing not-found.

Image requirement: the base image installs `clang clang-tools-extra` in its final
stage (a CONSUMER tool, like `cdb-compare`, NOT a toolchain - so it goes in the
final stage, not the toolchain-only builder), with a `clang-tidy --version` sanity
in the final stage. Every per-target image layers on this base, so the check needs
no per-target change; `BUILD_DIR` from config.env (reused as the sampler's
`--build-dir`) is the only per-target input.

### Consumer fault demonstration

The check must catch a deliberately broken entry: a stripped `-I` the tool then
rejects. Unlike the replay bad-directory fault (a host-side fixture in
`selftest.sh`), this demo MUST run where clang exists - IN the build container -
because the host has no compiler. It is a small, clearly-labeled in-container
one-off, `consumer-fault-demo.sh`, run by the maintainer against the curl target
image:

```sh
podman run --rm --systemd=always \
    -v tests/dogfooding/internal/consumer-loop.sh:/consumer-loop.sh:ro,Z \
    -v tests/dogfooding/internal/consumer-fault-demo.sh:/demo.sh:ro,Z \
    bear-dogfood-curl:<sha> sh /demo.sh
```

It builds curl (so the capture, sources, and headers exist), takes a KNOWN-GOOD
entry the consumer accepts (`altsvc.c` => OK), strips the `-I/src*` flags that
TU needs to find `curl/system.h`, and shows the SAME shipped `consumer_cdb`
function now reports FAIL for that entry. No JSON is hand-edited beyond removing
the one include group; the fault is a real, broken database entry. The demo
exits 0 iff the good entry was OK (rc 0) and the broken entry was caught (rc 1):

```
good: rc=0 (ok=1 fail=0 inconclusive=0)
bad:  rc=1 (ok=0 fail=1 inconclusive=0)
  diag: /src/lib/curl_setup.h:330:10: error: 'curl/system.h' file not found
DEMO PASSED
```

## dogfood-injected-fault

The invariants and replay checks must demonstrably catch injected faults - a
dropped entry, a duplicated entry, an empty/blank field, and a corrupted
`directory`. This is demonstrated WITHOUT a container by `selftest.sh` against
tiny, committed, hand-written fault fixtures under `faults/` (so the fault is
unambiguous and not the product of fragile in-shell JSON surgery on a real
capture):

- `faults/duplicate.json` - two byte-identical entries => `cdb-compare
  invariants` FAILS its no-true-duplicates check.
- `faults/empty-arguments.json` - an entry with `"arguments": []` => invariants
  FAILS non-empty-arguments.
- `faults/blank-directory.json` - an entry with `"directory": ""` => invariants
  FAILS non-empty-directory (a `cd ""` consumer lands in the wrong tree, so the
  entry is non-replayable). Distinct from `bad-directory.json`, which holds a
  nonexistent path to exercise REPLAY.
- `faults/undercount.json` - 2 entries asserted against `--expected-objects 3
  --tolerance 0` => invariants FAILS entry-count (the "dropped entry" fault).
- `faults/bad-directory.json` - a valid entry whose `directory` does not exist
  => the replay loop FAILS (the recorded compile can never run from where Bear
  claims it did - the "corrupted directory" fault). `faults/tu/hello.c` is the
  trivial TU it nominally compiles.

`selftest.sh` runs the host `cdb-compare invariants` against the invariants
fixtures and the SAME `replay-loop.sh` function (host-side) against the
bad-directory fixture, asserts each check exits non-zero (the fault was caught),
adds a control that an honest CDB passes (no false positive), and prints a clear
pass/fail. It needs no container, so it is fast. It is invoked as
`tests/dogfooding/internal/selftest.sh`.

Unlike the containerized gates, this is the one path that never builds an image,
so it runs a locally built `target/release/cdb-compare` on the host (built with
`cargo build --release -p bear-test-tools --bin cdb-compare`) - the sole host-
binary user in the harness. Because it is built and run in the same environment,
there is no cross-libc concern; if the binary is missing it exits with that build
hint.

## dogfood-metrics-collect

While Bear builds a target, the suite can profile `bear-driver`'s CPU and memory
with rprof (github.com/rizsotto/rprof) and keep the full capture as an artifact.
The profiled subject is `bear-driver` SPECIFICALLY - the component that
accumulates every intercepted execution and serializes the compilation database,
so its memory is the load-bearing scaling signal; the short-lived per-exec
preload and `bear-wrapper` are not profiled.

This is intentionally NOT automated: the harness only COLLECTS the rprof JSONL -
it never parses or summarizes it. The maintainer runs `rprof view` afterward to
render and compare runs.

Implementation: rprof (pinned to its `v1.0.0` release) is baked into the base
image via the existing multi-stage build - compiled in the toolchain-carrying
builder, the ~1 MB static binary copied into the toolchain-free final on PATH.
The `--metrics` flag turns the build into `rprof run -o /out/metrics.jsonl --
bear -- <build>`. Because the `bear` entry script execs `bear-driver` (same PID)
and rprof measures the single launched process WITHOUT following descendants,
this profiles exactly `bear-driver` and excludes the build's compiler processes -
no PID gymnastics needed. The whole JSONL is copied to
`results/<target>/<label>/metrics.jsonl` (determinism's two builds yield
`metrics.run1.jsonl` / `metrics.run2.jsonl`).

`--metrics` is an additive modifier: it layers on any mode (golden / oracle /
determinism / invariants / replay), and on a gate-less (`VALIDATION=none`) target
with no check mode it stands alone as a profiled build whose deliverable is the
artifact. The metrics file is advisory - a missing file warns, never fails - and
the wrapping is runner-controlled (rprof in the base is harmless when unused).

## Adding a new target

A target is a directory under `targets/<name>/` with a `config.env` and whatever
in-container build glue it needs. `config.env` fields (see the existing targets
for working examples):

| Field | Purpose |
|---|---|
| `VALIDATION` | `golden`, `oracle`, or `none`. Omit to default to `golden`. `none` means no default gate; the target runs only with an on-demand check. |
| `BASE_IMAGE` | The base image pinned by digest (dogfood-pinned-target). |
| `<NAME>_VERSION` / `<NAME>_URL` | The source revision, pinned by URL + sha256. |
| `BUILD_TYPE` | Release by default; pins the recorded flags. |
| `SRC_DIR` / `BUILD_DIR` / `OBJECTS_DIR` | Fixed container paths (dogfood-fixed-paths). `BUILD_DIR` is also the sampler's `--build-dir` for replay/consumer. |
| `OBJECT_COUNT_CMD` | Build-system-native object count for the invariants entry-count cross-check. Omit to use the default (`find` for `*.o` under `OBJECTS_DIR`). See dogfood-invariants for why this is per-target. |
| `OBJECT_TOLERANCE_PCT` | Tolerance band for entry-count. |
| `MIN_FREE_KIB` | Preflight free-disk minimum on the podman graphroot. |
| `TARGET_BUILD_CMD` | The in-container build command; thread `${INJECT_CFLAGS:-}` into its compiler flags so `--inject-fault` works (dogfood-determinism). |

For a golden target, add `goldens/<name>/compile_commands.json` by running
`run.sh --rebless <name>` once and committing the result. For an oracle target,
the build must emit its own reference database and the harness compares against
it -- no committed baseline. A `none` target needs neither; it is exercised only
by `--invariants` / `--determinism` / `--replay` / `--consumer`.

## Extending cdb-compare

Every comparison, normalization, and structural check lives in the
`cdb-compare` binary (package `bear-test-tools`, `crates`/`tests/tools`), not in
shell. That is deliberate: the logic is unit-tested in one place. When a target
surfaces a new benign difference (e.g. an oracle argument mismatch that
`--drop-dependency-flags` does not cover, or a new structural invariant), add a
tested rule to `cdb-compare` and select it with a flag from `run.sh` - do NOT
add a shell allow-list or a `jq` heuristic. See `tests/tools/SPEC.md` for the
comparator's normalization and matching model.
