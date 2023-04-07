#!/usr/bin/env sh

# RUN: cd %T; %{shell} %s %t
# RUN: assert_compilation %t.compilations.json count -eq 1
# RUN: assert_compilation %t.compilations.json contains -file /home/user/broken_build.c -directory /home/user -arguments /usr/bin/gcc -c -o broken_build.o broken_build.c
# RUN: %{citnames} --verbose --input %t.commands.json --output-compile %t.compilations.json --config %t.config.json --append
# RUN: assert_compilation %t.compilations.json count -eq 1
# RUN: assert_compilation %t.compilations.json contains -file /home/user/broken_build.c -directory /home/user -arguments /usr/bin/gcc -c -o broken_build.o broken_build.c

cat > "$1.config.json" << EOF
{
  "compilation": {
    "compilers_to_recognize": [
      {
        "executable": "/usr/bin/gcc"
      },
      {
        "executable": "/usr/bin/c++"
      }
    ]
  },
  "output": {
    "content": {
      "include_only_existing_source": false
    },
    "format": {
      "command_as_array": false,
      "drop_output_field": false
    }
  }
}
EOF

cat > "$1.compilations.json" << EOF
[
  {
    "arguments": [
      "/usr/bin/gcc",
      "-c",
      "-o",
      "broken_build.o",
      "broken_build.c"
    ],
    "directory": "/home/user",
    "file": "/home/user/broken_build.c",
    "files": [],
    "output": "/home/user/broken_build.o"
  }
]
EOF

cat > "$1.commands.json" << EOF
EOF
