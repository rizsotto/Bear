# Some compiler-adjacent names are deliberately not recognized

## Context

Several executables look like compilers, or sit next to them, but
recording them would add wrong or useless entries. Users and
contributors propose them from time to time.

- **`as`, the GNU assembler.** GCC and Clang spawn it on a temporary
  `.s` file for every ordinary compile. Each run would add a throwaway
  entry keyed on a name that changes every run (`/tmp/cc<random>.s`),
  which duplicate detection cannot collapse. Assembly compiled through
  a driver (`gcc -c foo.s`) is already recorded through the driver's
  own entry.
- **`mpirun`, `mpiexec`, and icecream's `icerun`.** They run arbitrary
  programs, not compilations. Recognizing them would record
  non-compilations.
- **`swift` and `zig`.** They dispatch on a subcommand (`swift build`,
  `zig cc`, `zig build`). The subcommand, not the executable name,
  selects the compiler. Recognition works by executable name, so this
  model does not fit.
- **`swift-frontend` and `swiftc -frontend`.** These are the Swift
  driver's internal per-file jobs. The user-facing `swiftc` invocation
  is the one to record.
- **`amdgpu-arch`.** It shares the `amd` prefix but reports GPU
  architectures; it does not compile.
- **`ml`, `ml64` (MASM).** Windows-only, and no user has asked for them.

## Decision

These names are not recognized, and invocations of them yield no
entry.

## Consequences

- Revisit `swift` and `zig` only with subcommand-aware recognition.
- Revisit MASM when a user needs it.
- The other exclusions follow from what the tools do. They should stay.

## References

- [recognition-compiler-names](../requirements/recognition-compiler-names.md),
  the exclusion list
- [recognition-compiler-launchers](../requirements/recognition-compiler-launchers.md),
  the `icerun` exclusion
- [#146](https://github.com/rizsotto/Bear/issues/146), compile-then-assemble
