#!/usr/bin/env sh

# RUN: cd %T; %{shell} %s %t
# RUN: %{citnames} --verbose --input %t.commands.json --output %t.compilations.json --config %t.config.json
# RUN: assert_compilation %t.compilations.json count -eq 1
# RUN: assert_compilation %t.compilations.json contains -file /home/user/broken_build.c -directory /home/user -arguments /usr/bin/wrapper -c -o broken_build.o broken_build.c -Dwrapper

cat > "$1.config.json" << EOF
{
  "compilation": {
    "compilers_to_recognize": [
      {
        "executable": "/usr/bin/wrapper",
        "additional_flags": ["-Dwrapper"]
      }
    ]
  },
  "output": {
    "content": {
      "include_only_existing_source": false
    },
    "format": {
      "command_as_array": true,
      "drop_output_field": false
    }
  }
}
EOF

cat > "$1.commands.json" << EOF
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
                    "/usr/bin/wrapper",
                    "-c",
                    "-o",
                    "broken_build.o",
                    "broken_build.c"
                ],
                "environment": {
                    "PATH": "/usr/local/bin:/usr/local/sbin:/usr/bin:/usr/sbin"
                },
                "program": "/usr/bin/wrapper",
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
