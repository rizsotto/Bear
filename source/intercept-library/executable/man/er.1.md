% BEAR(1) Bear User Manuals
% László Nagy
% Feb 24, 2020

# NAME

er - execution reporter

# SYNOPSIS

er [*options*] [*build command*]

# DESCRIPTION

Supervise a program execution and report metrics about it.

# OPTIONS

\--help
:   Prints a short help message about the usage and exits.

\--version
:	Prints out `er` version number and exits. 

\--verbose
:	Enables verbose output from `er`. (optional)

\--destination
:   Specifies the directory path for the execution reports. (required)

\--library
:   Full path to the intercept library. (optional)

\--execute
:   Full path to the executable to supervise. (required)

\--
:   The list of arguments to the executable. (required)
    The first argument (by convention) is the executable name.

# OUTPUT

If verbose execution is not requested, it shall not print out to the console.

The execution reports are files created into the given execution directory.

# EXIT STATUS

The exit status is the exit status of the build command.
Except when er crashes, then it sets to non zero.

# ENVIRONMENT

Ignores the environment variables, uses only the received command line
parameters. It sets the following environment variables for the supervised process:

`INTERCEPT_REPORT_DESTINATION`
:	Temporary directory to collect the execution reports at one place.
	Directory path is derived from `TMPDIR`, `TEMP` or `TMP` environment
	variable.

`INTERCEPT_LIBRARY`
:   An absolute path to the execution intercept library (`libexec.so` or `libexec.dylib`).

`INTERCEPT_REPORT_COMMAND`
:   An absolute path to the program itself.

`INTERCEPT_VERBOSE`
:   Verbosity level of the interception.

# FILES

`libexec.so` or `libexec.dylib`
:	The preload library which implements the *exec* methods.

# SEE ALSO

ld.so(8), exec(3)

# BUGS

TODO

# COPYRIGHT

Copyright (C) 2012-2020 by László Nagy
<https://github.com/rizsotto/Bear>
