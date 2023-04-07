#!/usr/bin/env sh

# UNSUPPORTED: true
# RUN: cd %T; %{shell} %s %t.commands.json
# RUN: %{citnames} --verbose --input %t.commands.json --output-compile %t.compilations.json
# RUN: assert_compilation %t.compilations.json count -eq 0

cat > $1 << EOF
{
    "context": {
        "host_info": {
            "_CS_GNU_LIBC_VERSION": "glibc 2.31",
            "_CS_GNU_LIBPTHREAD_VERSION": "NPTL 2.31",
            "_CS_PATH": "/usr/bin",
            "machine": "x86_64",
            "release": "5.8.4-200.fc32.x86_64",
            "sysname": "Linux",
            "version": "#1 SMP Wed Aug 26 22:28:08 UTC 2020"
        },
        "intercept": "library preload"
    },
    "executions": [
        {
            "command": {
                "arguments": [
                    "/usr/bin/bash",
                    "/home/user/build.sh"
                ],
                "environment": {
                    "PATH": "/usr/local/bin:/usr/local/sbin:/usr/bin:/usr/sbin"
                },
                "program": "/usr/bin/bash",
                "working_dir": "/home/user"
            },
            "run": {
                "events": [
                    {
                        "at": "2020-09-13T21:13:04.724530Z",
                        "type": "started"
                    },
                    {
                        "at": "2020-09-13T21:13:04.798790Z",
                        "status": 0,
                        "type": "terminated"
                    }
                ],
                "pid": 629422,
                "ppid": 629392
            }
        }
    ]
}
EOF
