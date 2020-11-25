% BEAR(1) Bear User Manuals
% László Nagy
% May 10, 2019

# NAME

Bear - Build EAR

# SYNOPSIS

bear [*options*] \-\- [*build command*]

# DESCRIPTION

Bear is a tool to generate compilation database for clang tooling.

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

\--include *directory*
:   Only include this directories or files to the output. The flag can
    be used multiple times. The directory is either absolute or
    relative to current working directory. Use `--exclude` to filter
    entries out from these directory.

\--exclude *directory*
:   Exclude these directories or files from the output. The flag can
    be used multiple times. The directory is either absolute or
    relative to current working directory. The `--include` will
    not enable entries from these directories.

\--force-preload
:   Force to use the dynamic linker method of `intercept` command.

\--force-wrapper
:   Force to use the compiler wrapper method of `intercept` command.

# OUTPUT

The JSON compilation database definition changed over time. The current
version of Bear generates entries where:

`directory`
:	has absolute path.

`file`
:	has absolute path.

`output`
:   has absolute path.

`arguments`
:   used instead of `command` to avoid shell escaping problems. (Configuration
    can force to emit the `command` field.) The compiler as the first argument
    has absolute path. Some non compilation related flags are filtered out from
    the final output.

# EXIT STATUS

The exit status of the program is the exit status of the build command.
Except when the program itself crashes, then it sets to non zero.

# SEE ALSO

intercept(1), citnames(1)

# COPYRIGHT

Copyright (C) 2012-2020 by László Nagy
<https://github.com/rizsotto/Bear>
