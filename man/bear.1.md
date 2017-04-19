% BEAR(1) Bear User Manuals
% László Nagy
% April 20, 2017

# NAME

Bear - Build EAR

# SYNOPSIS

bear [*options*] [*build command*]

# DESCRIPTION

Bear is a tool to generate compilation database for clang tooling.

The JSON compilation database
<http://clang.llvm.org/docs/JSONCompilationDatabase.html> is used in
Clang project to provide information how a single compilation unit
was processed. When that is available then it is easy to re-run the
compilation with different programs.

Bear executes the original build command and intercepts the subsequent
execution calls. To achieve that Bear uses library preload mechanism
provided by the dynamic linker.
There is a library which defines the *exec* methods and used in every
child processes of the build command.
The executable itself sets the environment up to child processes and
writes the output file.

# OPTIONS

\--version
:	Print out Bear version number.

-v, \--verbose
:	Enable verbose output from Bear. A second, third and fourth flags
	increases verbosity.

-o *file*, \--cdb *file*
: 	Specify output file. (Default value provided.) The output is not
	continuously updated, it's done when the build command finished.

\--use-cc *program*
:	Hint Bear to classify the given program name as C compiler.

\--use-c++ *program*
:	Hint Bear to classify the given program name as C++ compiler.

-a, \--append
:	Use previously generated output file and append the new entries to it.
	This way you can run Bear continuously during work, and it keeps the
	compilation database up to date. File deletion and addition are both
	considered. But build process change (compiler flags change) might
	cause duplicate entries.

-l *path*, \--libear *path*
:	Specify the preloaded library location. (Default value provided.)

# OUTPUT

The JSON compilation database definition changed over time. The current
version of Bear generates entries where:

`directory`
:	has absolute path.

`file`
:	has relative path to the `directory`.

`arguments`
:	used instead of `command` to avoid shell escaping problems. The source
    file in the compiler call match to the `file` attribute, therefore
	it is relative path to `directory`. Other filesystem related references
	are not modified (those still can be absolute or relative depending the
	original command).

Some non compilation related flags are filtered out from the final output.

# EXIT STATUS

Bear exit status is the exit status of the build command.
Except when bear crashes, then it sets to non zero.

# ENVIRONMENT

`INTERCEPT_BUILD_TARGET_DIR`
:	Temporary directory to collect the execution reports at one place.
	Directory path is derived from `TMPDIR`, `TEMP` or `TMP` environment
	variable.

`LD_PRELOAD`
:	Used by the dynamic loader on Linux, FreeBSD and other UNIX OS.
	Value set by Bear, overrides previous value for child processes.

`DYLD_INSERT_LIBRARIES`
:	Used by the dynamic loader on OS X.
	Value set by Bear, overrides previous value for child processes.

`DYLD_FORCE_FLAT_NAMESPACE`
:	Used by the dynamic loader on OS X.
	Value set by bear, overrides previous value for child processes.

# FILES

`libear.so` or `libear.dylib`
:	The preload library which implements the *exec* methods.

# SEE ALSO

ld.so(8), exec(3)

# BUGS

Because Bear uses `LD_PRELOAD` or `DYLD_INSERT_LIBRARIES` environment variables,
it does not append to it, but overrides it. So builds which are using these
variables might not work. (I don't know any build tool which does that, but
please let me know if you do.)

Security extension/modes on different operating systems might disable library
preloads. This case Bear behaves normally, but the result compilation database
will be empty. (Please make sure it's not the case when reporting bugs.)
Notable examples for enabled security modes are: SIP on OS X Captain and
SELinux on Fedora, CentOS, RHEL.

# COPYRIGHT

Copyright (C) 2012-2017 by László Nagy
<https://github.com/rizsotto/Bear>
