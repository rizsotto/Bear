#!/usr/bin/env sh

# RUN: cd %T; %{shell} %s %t
# RUN: %{citnames} --verbose --input %t.commands.json --output %t.compilations.json --config %t.config.json
# RUN: assert_compilation %t.compilations.json count -eq 1
# RUN: assert_compilation %t.compilations.json contains -file /home/user/broken_build.c -directory /home/user -arguments /usr/bin/wrapper -c -Dwrapper -o broken_build.o broken_build.c

cat > "$1.config.json" << EOF
{
  "citnames": {
    "compilation": {
      "compilers_to_recognize": [
        {
          "executable": "/usr/bin/wrapper",
          "flags_to_add": ["-Dwrapper"],
          "flags_to_remove": ["-Wall"]
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
}
EOF

cat << EOF | tr '\r\n' ' ' > "$1.commands.json"
{
  "rid": "13711651845693228889",
  "started": {
    "execution": {
      "executable": "/usr/bin/wrapper",
      "arguments": [
        "/usr/bin/wrapper",
        "-c",
        "-o",
        "broken_build.o",
        "broken_build.c",
        "-Wall"
      ],
      "working_dir": "/home/user",
      "environment": {
        "PATH": "/usr/local/bin:/usr/local/sbin:/usr/bin:/usr/sbin"
      }
    },
    "pid": 380296,
    "ppid":380286
  },
  "timestamp": "2021-07-17T02:59:36.338446Z"
}
EOF

echo "" >> "$1.commands.json"

cat << EOF | tr '\r\n' ' ' >> "$1.commands.json"
{
  "rid": "13711651845693228889",
  "terminated": {
    "status": "0"
  },
  "timestamp": "2021-07-17T02:59:36.344702Z"
}
EOF

echo "" >> "$1.commands.json"
