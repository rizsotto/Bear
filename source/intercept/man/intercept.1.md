% INTERCEPT(1) Bear User Manuals
% László Nagy
% Sep 14, 2020

# NAME

intercept - intercept command executions in user space.

# SYNOPSIS

intercept [*options*] \-\- [*build command*]

# DESCRIPTION

For intercepting the compiler executions, intercept uses the `LD_PRELOAD`
or `DYLD_INSERT_LIBRARIES` mechanisms provided by the dynamic linker.
When the dynamic linker is not working (because the executable is not a
dynamically linked executable or security protection disables the linker)
then intercept uses compiler wrappers to record the compiler calls. The
wrapper sends execution report and calls the real compiler. (Not only
compilers, but linkers, assemblers and other tools are also wrapped.)

The reports are collected by the `intercept` over a gRPC interface, and
digested into an output JSON file.

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
:   Force to use the dynamic linker method to intercept the children
    processes.

\--force-wrapper
:   Force to use the compiler wrapper method to intercept the children
    processes.

# EXIT STATUS

The exit status of the program is the exit status of the build command.
Except when the program itself crashes, then it sets to non zero.

# OUTPUT FILE

It's a JSON file, with the command execution history. (Plus some metadata, that
is useful for debugging the application.)

```json
{
  "context": {
    "host_info": {
      "_CS_GNU_LIBC_VERSION": "glibc 2.30",
      "_CS_GNU_LIBPTHREAD_VERSION": "NPTL 2.30",
      "_CS_PATH": "/usr/bin",
      "machine": "x86_64",
      "release": "5.5.13-200.fc31.x86_64",
      "sysname": "Linux",
      "version": "#1 SMP Wed Mar 25 21:55:30 UTC 2020"
    },
    "intercept": "library preload"
  },
  "executions": [
    {
      "command": {
        "arguments": [
          "sleep",
          "1"
        ],
        "environment": {
          "PATH": "/usr/local/bin:/usr/local/sbin:/usr/bin:/usr/sbin"
        },
        "program": "/usr/bin/sleep",
        "working_dir": "/home/lnagy/Code/Bear.git"
      },
      "run": {
        "events": [
          {
            "at": "2020-02-16T21:00:00.000Z",
            "type": "start"
          },
          {
            "at": "2020-02-16T21:00:00.000Z",
            "status": 0,
            "type": "stop"
          }
        ],
        "pid": 503092,
        "ppid": 503083
      }
    }
  ]
}
```

# TROUBLESHOOTING

The potential problems you can face with are: the build with and without the
interception behaves differently (eg.: the build crash with the `intercept`
tool, but succeed otherwise). The output is empty and it failed to intercept
the children process execution by the build command.

There could be many reasons for any of these failures. It's better to consult
with the project wiki page for known problems, before open a bug report.

The most common cause for empty outputs is that the build command was not
executed any commands. The reason for that could be, because incremental builds
not running the compilers if everything is up to date. Remember, `intercept`
is not understanding the build file (eg.: makefile), but intercepts the executed
commands.

# SEE ALSO

bear(1)

# COPYRIGHT

Copyright (C) 2012-2021 by László Nagy
<https://github.com/rizsotto/Bear>
