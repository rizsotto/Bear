---
title: Interception mode selection
status: implemented
---

## Intent

The user runs `bear -- <build>` without choosing how the interception
happens. Bear has two interception mechanisms -- injecting a shared
library into the build's processes (`interception-preload-mechanism`)
and placing wrapper executables on PATH
(`interception-wrapper-mechanism`) -- and selects exactly one of them
per invocation, before the build starts. The default fits the platform;
a configuration option forces either mode for users who need the other
one.

## Acceptance criteria

- Exactly one interception mechanism is active per invocation: the
  selection happens before the build starts and does not change
  mid-build or per intercepted process
- Without configuration, preload is selected on Linux and BSD systems,
  and wrapper is selected on macOS and Windows
- A configuration option forces either mode, overriding the platform
  default (for example wrapper on Linux, or preload on macOS with
  System Integrity Protection disabled)
- When the selected mode is preload but library injection is
  unavailable on the host -- on Windows, or on macOS while System
  Integrity Protection is enabled (checked at startup) -- Bear reports
  an error that names wrapper mode as the alternative and does not run
  the build; it never silently substitutes the other mode

## Testing

Given a Linux host and no interception configuration:

> When the user runs `bear intercept --output events.json -- sh build.sh`,
> then the shell and the processes it spawns are all reported --
> evidence that the preload mechanism is active, since wrapper mode
> never observes the shell.

Given a macOS or Windows host and no interception configuration:

> When the user runs `bear -- make`,
> then the wrapper directory is present in the working directory while
> the build runs -- evidence that the wrapper mechanism is the platform
> default there.

Given a Linux host and a configuration forcing wrapper mode:

> When the user runs `bear --config config.yaml -- sh build.sh`,
> then the wrapper directory is present in the working directory while
> the build runs -- evidence that the wrapper mechanism is active even
> though the platform default is preload.

Given a macOS host with System Integrity Protection enabled and a
configuration forcing preload mode:

> When the user runs `bear -- make`,
> then Bear exits with an error suggesting wrapper mode,
> and the build is not started.

## Notes

- The mechanics and limitations of each mode are owned by
  `interception-preload-mechanism` and `interception-wrapper-mechanism`.
- GitHub issue #609 records user confusion over choosing a mode; the
  platform default covers the common case and the configuration
  override the rest.
