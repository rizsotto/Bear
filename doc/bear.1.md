% BEAR(1) Bear User Manuals
% László Nagy
% January 8, 2013

# NAME

bear - build ear

# SYNOPSIS

bear [*options*] -- [*build commands*]

# DESCRIPTION

Bear is a tool to generate compilation database for clang tooling.

The [JSON compilation database][1] is used in clang project to provide
information how a single compilation unit was processed. When that
is available then it is easy to re-run the compilation with different
programs.

Bear execs the original build command and intercept the `exec` calls.
To achive that Bear uses the `LD_PRELOAD` or `DYLD_INSERT_LIBRARIES`
mechanisms provided by the dynamic linker. There is a library which
defines the `exec` methods and used in every child processes of the
build command. The executable itself sets the environment up to child
processes and writes the output file.

# OPTIONS

-c *config*
:   Specify config file. Default value provided.

-o *output*
:   Specify output file. Default value provided.
    (This option mainly for development purposes.)

-l *library*
:   Specify the preloaded library location. Default value provided.
    (This option mainly for development purposes.)

-s *socket*
:   Specify UNIX socket file. Default value provided.
    (This option mainly for development purposes.)

-d
:   Generate debug output. Debug output is also a *JSON* formated file.
    It contains all available informations of the `exec` calls.
    (This option for those who want to extend functionality of bear.)

-v
:   Print out Bear version and exit.

# OUTPUT

There are two version of output formats. One is defined by the clang
tooling. This goes like this:

    [
      { "directory": "/home/user/llvm/build",
        "command": "clang++ -Irelative -c -o file.o file.cc",
        "file": "file.cc" },
      ...
    ]

To achive this bear has to run some filtering and formating.
Build tools exec many commands during the build process.
Bear has to find was that a compiler call, and what was the source file?

Filtering has three steps. It checks the executable name agains the
known compiler names. It checks any of the arguments to match as source
file. (Usually done by extension match.) And checks that there is no
parameter which might considered as non compilation. (Like dependency
file generation, which migth cause duplicates in the output.)

Since the post process might be buggy, there is a way to see all exec
calls. This gives opportunity to write custom post processing. The format
of the debug output looks like this:

    [
      { "pid": "1234",
        "ppid": "100",
        "function": "execve",
        "directory": "/home/user/llvm/build",
        "command": "clang++ -Irelative -c -o file.o file.cc" },
      ...
    ]

Both output is JSON format, which means that *command* field is escaped
if one of the argument had space, slash or quote character. All the other
fields are as it was captured.

# BUGS

Compiler wrappers like [ccache][2] and [distcc][3] could cause duplicates
or missing items in the compilation database. Make sure you have been disabled
before you run Bear.

# COPYRIGHT

Copyright (C) 2012, 2013 by László Nagy <https://github.com/rizsotto/Bear>

[1]: http://clang.llvm.org/docs/JSONCompilationDatabase.html
[2]: http://ccache.samba.org/
[3]: http://code.google.com/p/distcc/
