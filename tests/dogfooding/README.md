# Bear dogfooding harness

A non-automated, release-time test harness. It runs Bear's *installed release*
binaries against a real open-source project, pinned to a known revision, inside
a throwaway container, then validates the `compile_commands.json` Bear captured.
It proves the end-to-end interception loop works on a real build and catches
regressions and correctness divergences in Bear's output before a release.

Nothing here touches your repo working tree, your devcontainer, or any cargo
cache: the sources and toolchain live only inside the per-run container.

This file is for running the harness. The design and per-check rationale (why
each check works the way it does, and how to extend one) live in `SPEC.md`.

## Targets and checks

The harness ships five targets. Each has a default way of validating a capture:

| Target | Build system | Default validation |
|---|---|---|
| `zlib` | autotools | Golden regression (compare against a committed baseline) |
| `curl` | CMake | Oracle (compare against the database CMake itself emits) |
| `ffmpeg` | custom `configure` | None -- run with an on-demand check below |
| `kernel` | Kbuild | None -- run with an on-demand check below |
| `nasm` | x264, custom `configure` | None -- run with an on-demand check below |

On top of the default, every target supports these on-demand checks, each
selected by a flag. They need no baseline, so they run against any target:

| Flag | Checks that... |
|---|---|
| `--determinism` | two identical builds produce equivalent captures |
| `--invariants` | the capture is structurally well-formed (no empty/duplicate entries; entry count matches objects built) |
| `--replay[=N]` | the compiler re-accepts a sample of recorded commands |
| `--consumer[=N]` | a real clang tool (clang-tidy) can parse a sample of entries |

`ffmpeg`, `kernel`, and `nasm` have no default gate (no golden, no CMake
oracle), so they are run *only* with the on-demand checks; a plain run against
them is rejected with a pointer to those flags. `ffmpeg` and `kernel` are scale
targets. `nasm` exercises a specific toolchain on a real build:

- `nasm` builds x264 (nasm-heavy, mixing nasm and gcc) to confirm Bear records
  a well-formed entry for each nasm assembly compile alongside the C ones. x264
  compiles every bit-depth-dependent source twice (8-bit and 10-bit), so this
  target runs Bear with a config (`targets/nasm/bear.yaml`) that adds
  `arguments` to the duplicate match key, keeping both compiles; the
  `--invariants` entry count then matches the on-disk object count exactly.

## Prerequisites

- **Rootless Podman** (developed against podman 5.8.3). The build container runs
  with `--systemd=always` so Bear's cgroup-based process-tree teardown works;
  this needs the host's delegated cgroup controllers
  (`/etc/systemd/system/user@.service.d/delegate.conf` with
  `Delegate=cpu cpuset io memory pids`).

- **Free disk** on the podman graphroot (zlib needs ~2 GiB, curl ~4 GiB for the
  images plus the build tree). The harness preflight checks this against each
  target's `MIN_FREE_KIB`.

The first run builds two cached images (`bear-dogfood-base:<sha>` and
`bear-dogfood-<target>:<sha>`, tagged by the Bear commit under test); later runs
reuse them. The base build compiles Bear from `git archive HEAD`, so the first
run takes a few minutes.

## How to run

All commands are run from the repo root.

```sh
# Default validation for a target (golden for zlib, oracle for curl).
tests/dogfooding/run.sh                 # zlib is the default target
tests/dogfooding/run.sh curl

# On-demand checks (any target; these skip the golden/oracle gate).
tests/dogfooding/run.sh --determinism zlib     # build twice, compare
tests/dogfooding/run.sh --invariants curl      # structural well-formedness
tests/dogfooding/run.sh --replay zlib          # replay 20 recorded commands
tests/dogfooding/run.sh --replay=30 curl       # replay a sample of 30
tests/dogfooding/run.sh --consumer curl        # feed 20 entries to clang-tidy
tests/dogfooding/run.sh --invariants kernel    # scale target, checks only

# Modifiers.
tests/dogfooding/run.sh --label rc1 curl       # name the run (results/curl/rc1/)
tests/dogfooding/run.sh --keep                 # keep the container for inspection
tests/dogfooding/run.sh --metrics ffmpeg       # also profile bear-driver (see below)

# Prove the checks actually catch faults, without a container (fast).
# This one runs the comparator on the host, so build it first:
#   cargo build --release -p bear-test-tools --bin cdb-compare
tests/dogfooding/internal/selftest.sh
```

Run artifacts land under `results/<target>/<label>/` (git-ignored). Goldens
live under `goldens/<target>/` and are tracked.

## Outcomes and exit codes

Every run prints one final `OUTCOME:` line and exits with:

| Outcome | Exit | Meaning |
|---|---|---|
| PASS | 0 | The check passed. No regression. |
| FAIL | 1 | The check failed -- a real behavioral change or defect in Bear's output. Read the artifact for that check (below) and either fix Bear or, for the golden, rebless. |
| INCONCLUSIVE | 2 | The target build failed for its own reasons (source fetch, sha, network, configure/make, OOM), or a sampling check verified nothing (every sampled entry was inconclusive). Not a Bear regression. |
| ERROR | 3 | Harness or Bear-infrastructure failure: podman missing, disk/digest preflight, base or target image build, an empty capture (a `libexec` / `INTERCEPT_LIBDIR` mismatch), or an oracle that matched 0 units. |

## Understanding a FAIL

Each check writes a human-readable artifact under `results/<target>/<label>/`.
On a FAIL, start there:

| Check | What a FAIL means | Read |
|---|---|---|
| Golden | The capture drifted from the committed baseline. | `git diff` on the golden; if the change is intentional, rebless (below) |
| Oracle | Matched units diverge from CMake's own database. | the `matched but differing` section of `oracle-report.txt` |
| Determinism | Two identical builds produced different captures -- real non-determinism or a race in Bear. | `determinism-diff.txt` |
| Invariants | Bear produced a malformed database (empty/duplicate entry, or the entry count is off from objects built). | `invariants-report.txt` |
| Replay | A recorded command would not replay -- a malformed entry (wrong cwd or mangled flag). | `replay_result` |
| Consumer | clang-tidy rejected a well-formed entry -- a semantically broken database (e.g. a wrong or missing `-I`). | `consumer_result` |

## Reblessing the golden

The golden is a frozen, normalized capture -- a change-detector, not a proof of
correctness. When a behavior change is *intentional* (Bear deliberately changed
the flags it records, or the pinned zlib/base moved), regenerate it on purpose:

```sh
tests/dogfooding/run.sh --rebless zlib
```

This runs the full pipeline and then, instead of gating, overwrites
`goldens/zlib/compile_commands.json` with the fresh normalized capture and
reports "reblessed" (exit 0). The new golden is left in the working tree for you
to:

1. Inspect the diff (`git diff tests/dogfooding/goldens/zlib/`) and confirm the
   change is the one you intended.
2. Commit it with a message explaining *why* the recorded behavior changed.

Reblessing is never automatic -- a normal run only ever reads the golden and
fails on mismatch, so an unintended change cannot silently overwrite it. Oracle
targets (curl) have no golden and reject `--rebless`; the CMake reference renews
itself when curl updates.

## Metrics: profiling bear-driver

`--metrics` additionally profiles `bear-driver`'s CPU and memory with
[rprof](https://github.com/rizsotto/rprof) while it builds the target, and keeps
the full JSONL at `results/<target>/<label>/metrics.jsonl` (determinism yields
`metrics.run1.jsonl` / `metrics.run2.jsonl`). It layers on any run:

```sh
tests/dogfooding/run.sh --metrics ffmpeg               # profiled build, metrics only
tests/dogfooding/run.sh --metrics --invariants curl    # invariants plus a profile
tests/dogfooding/run.sh --metrics --determinism zlib   # two builds, two profiles
```

The harness only *collects* the file; render and compare runs yourself with
`rprof view`. It is harmless when `--metrics` is not given.

## What the harness does NOT do

- It does not modify the repo working tree, the devcontainer image, or any cargo
  cache. Sources and toolchain live only in the throwaway container.
- It does not leave its per-run container behind (unless `--keep`). It does keep
  the two cached images so reruns are fast; remove them when done with
  `podman rmi bear-dogfood-<target>:<sha> bear-dogfood-base:<sha>`.
- It does not parse or summarize the metrics profile -- that is `rprof view`'s
  job.
