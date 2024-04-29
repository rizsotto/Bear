% BEAR-INTERCEPT(1) Bear User Manuals
% László Nagy
% Sep 04, 2021

# NAME

bear-intercept - intercept command executions in user space.

# SYNOPSIS

bear intercept [*options*] \-\- [*build command*]

# DESCRIPTION

The command executes the given build command and generates an output
file which contains all process execution related events has happened
during the build.

The process execution events are: start, signal, termination. The output
will contain only the child processes. Depending on the interception mode
the output might only contain a subset of the executed commands.
Interception modes are:

- _preload_ uses the `LD_PRELOAD` or `DYLD_INSERT_LIBRARIES` mechanisms
  provided by the dynamic linker. The pre-loaded library hijacks the
  process execution calls, and executes a supervisor process, which reports
  the execution. The method fails when the executable statically linked,
  or security protection disables the dynamic linker.

- _wrapper_ mode interpose a wrapper program to the build. The wrapper
  sends execution report and calls the original program. The method fails
  when the build system is not flexible enough for interposing build
  tools.

The reports are collected by the `intercept` over a gRPC interface, and
written into an output file.

# OPTIONS

\--version
:	Print version number.

\--help
:   Print help message.

\--verbose
:   Enable verbose logging.

\--output *file*
:   Specify output file. (Default file name provided.) The output is a
    command execution list, with some extra information. The syntax
    is detailed in a separate section.

\--force-preload
:   Force to use the preload method to intercept the children processes.

\--force-wrapper
:   Force to use the wrapper method to intercept the children processes.

# EXIT STATUS

The exit status of the program is the exit status of the build command.
Except when the program itself crashes, then it sets to non-zero.

# OUTPUT FILE

The output file has [JSON lines](https://jsonlines.org/) format, where each
line terminated with `\n` line separator and each line is a JSON object.

The JSON objects are process execution events: process start, process got
signal, process terminated. (For the schema of these events, please consult
with the source code of this project.)

# TROUBLESHOOTING

The potential problems you can face with are: the build with and without the
interception behaves differently (eg.: the build crash with the intercept
tool, but succeed otherwise). The output is empty, and it failed to intercept
the children process execution by the build command.

The most common cause for empty outputs is that the build command did not
execute any commands. The reason for that could be, because incremental builds
not running the compilers if everything is up-to-date. Remember, this program
does not understand the build file (eg.: makefile), but intercepts the executed
commands.

There could be many reasons for any of these failures. It's better to consult
with the project wiki page for known problems, before open a bug report.

# SEE ALSO

`bear(1)`

# COPYRIGHT

Copyright (C) 2012-2024 by László Nagy
<https://github.com/rizsotto/Bear>
