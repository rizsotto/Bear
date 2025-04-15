% BEAR(1) Bear User Manuals
% László Nagy
% Jan 02, 2023

# NAME

Bear - a tool to generate compilation database for Clang tooling.

# SYNOPSIS

bear [*options*] \-\- [*build command*]

# DESCRIPTION

The JSON compilation database
<http://clang.llvm.org/docs/JSONCompilationDatabase.html> is used in
Clang project to provide information how a single compilation unit
was processed. When that is available then it is easy to re-run the
compilation with different programs.

Bear executes the original build command and intercept the command
executions issued by the build tool. From the log of command executions
it tries to identify the compiler calls and creates the final
compilation database.

# OPTIONS

\--version
:   Print version number.

\--help
:   Print help message.

\--verbose
:   Enable verbose logging.

\--output *file*
:   Specify output file. (Default file name provided.) The output is
    a JSON compilation database.

\--append
:   Use previously generated output file and append the new entries to it.
    This way you can run Bear continuously during work, and it keeps the
    compilation database up to date. File deletion and addition are both
    considered. But build process change (compiler flags change) might
    cause duplicate entries.

\--config *file*
:   Specify a configuration file. The configuration file captures how
    the output should be formatted and which entries it shall contain.

\--force-preload
:   Force to use the dynamic linker method of `intercept` command.

\--force-wrapper
:   Force to use the compiler wrapper method of `intercept` command.

\--enable-network-proxy
:   Forward HTTP proxy environment variables (`http_proxy`, `https_proxy`,
    `grpc_proxy` and their capitalized versions) to `intercept` command.
    They are unset by default.

# COMMANDS

`bear-intercept(1)`
:   Intercepts events that happened during the execution of the build
    command.

`bear-citnames(1)`
:   Deduce the semantics of the commands captured by `bear-intercept(1)`.

# OUTPUT

The JSON compilation database definition changed over time. The current
version of Bear generates entries where:

`directory`
:   has absolute path.

`file`
:   has absolute path.

`output`
:   has absolute path.

`arguments`
:   used instead of `command` to avoid shell escaping problems. (Configuration
    can force to emit the `command` field.) The compiler as the first argument
    has absolute path. Some non compilation related flags are filtered out from
    the final output.

# CONFIG FILE

Read `bear-citnames(1)` man page for the content of this file. `bear` is not
reading the content of this file, but passing the file name to `bear citnames`
command.

# EXIT STATUS

The exit status of the program is the exit status of the build command.
Except when the program itself crashes, then it sets to non-zero.

# TROUBLESHOOTING

The potential problems you can face with are: the build with and without Bear
behaves differently or the output is empty.

The most common cause for empty outputs is that the build command did not
execute any commands. The reason for that could be, because incremental builds
not running the compilers if everything is up-to-date. Remember, Bear does not
understand the build file (eg.: makefile), but intercepts the executed
commands.

The other common cause for empty output is that the build has a "configure"
step, which captures the compiler to build the project. In case of Bear is
using the _wrapper_ mode (read `bear-intercept(1)` man page), it needs to
run the configure step with Bear too (and discard that output), before run
the build with Bear.

There could be many reasons for any of these failures. It's better to consult
with the project wiki page for known problems, before open a bug report.

# COPYRIGHT

Copyright (C) 2012-2024 by László Nagy
<https://github.com/rizsotto/Bear>
