---
id: interception-001
title: LD_PRELOAD-based command interception
status: implemented
tests:
  - test_basic_c_compilation
---

## Intent

On Linux and macOS, Bear intercepts compiler invocations by injecting a shared
library into the build process via `LD_PRELOAD` (Linux) or `DYLD_INSERT_LIBRARIES`
(macOS). The intercepted commands are reported to the Bear collector for processing.

## Acceptance criteria

- `exec` family calls, `posix_spawn`, `popen`, and `system` are intercepted
- Child processes inherit the interception environment
- Intercepted commands are reported to the TCP collector
- The build process completes normally (interception is transparent)
- Internal compiler invocations (e.g., `cc1`) are filtered out

## Non-functional constraints

- Must not alter build output or exit codes
- Must handle concurrent builds (parallel make)
- Platform: Linux (`LD_PRELOAD`), macOS (`DYLD_INSERT_LIBRARIES`)
