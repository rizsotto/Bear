---
title: Header file entries in the output
status: implemented
---

## Intent

Editors and linters that consume `compile_commands.json` frequently look up
compile flags for header files, not only for translation units, so that
features like "jump to definition" or "find references" work when a header is
open. A user can opt in to having header files receive a synthesized entry
derived from a compiled translation unit. This is off by default: without
configuration the output is byte-identical to today.

## Acceptance criteria

- A configuration option enables header entries and selects a discovery
  strategy. When enabled, header files receive a synthesized entry whose flags
  are derived from a compiled translation unit.
- Two discovery strategies are available, chosen by configuration:
  - A default strategy considers header files that sit in the same directory
    as a compiled source.
  - An opt-in strategy reads the dependency files the build already emitted
    to find the exact headers each translation unit included, scoped to those
    that resolve inside the compilation's own working directory.
- A synthesized entry clones a compiled translation unit's arguments with the
  source path replaced by the header path and the output-file flag removed;
  the synthesized entry has no output field.
- Which files count as headers is fixed (a built-in header-extension set); it
  is not user-configurable.
- Only translation units for C, C++, and Objective-C sources are eligible to
  donate their arguments to a synthesized header entry.
- Synthesized entries pass through duplicate detection and validation like any
  other entry; a header that already has a real entry in the database is not
  duplicated.
- When the option is disabled (the default), no synthesized entries appear in
  the output.

## Non-functional constraints

Streaming is preserved: memory use is proportional to the number of
directories considered, not to the number of entries in the database.
Directory scanning reads each directory at most once regardless of how many
translation units or headers it contains.

## Testing

Given a build that compiles `src/main.c`, which sits next to an uncompiled
`src/util.h`:

> When Bear generates the database with header entries disabled (the default),
> then the database contains an entry for `src/main.c` only.

Given the same build:

> When Bear generates the database with header entries enabled using the
> default (same-directory) strategy,
> then the database also contains an entry for `src/util.h`
> whose arguments are `src/main.c`'s with the source path swapped for the
> header path and the output-file flag removed,
> and which has no output field.

For that second scenario, the source entry and its synthesized header entry
are a pair like:

```json
[
  {
    "directory": "/home/user/project",
    "file": "src/main.c",
    "arguments": ["cc", "-c", "src/main.c", "-o", "src/main.o"]
  },
  {
    "directory": "/home/user/project",
    "file": "src/util.h",
    "arguments": ["cc", "-c", "src/util.h"]
  }
]
```

Given a build whose compilation emitted a dependency file listing the headers
it included, some inside the compilation's working directory and some (system
headers) outside it:

> When Bear generates the database with the dependency-files strategy,
> then the database contains a synthesized entry for exactly the in-scope
> headers that dependency file lists, and no others.

## Rationale

- [Header entry synthesis by sibling cloning](../rationale/header-synthesis-sibling-clone.md)
