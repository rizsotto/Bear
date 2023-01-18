#!/usr/bin/env sh

# REQUIRES: shell
# RUN: %{shell} %s %t
# RUN: cd %T; %{bear} --verbose --output %t.json --config %t/config.json -- %{shell} %t/build.sh
# RUN: assert_compilation %t.json count -eq 2
# RUN: assert_compilation %t.json contains -file %t/source_1.c -directory %T -arguments %{c_compiler} -c -I. %t/source_1.c
# RUN: assert_compilation %t.json contains -file %t/source_2.c -directory %T -arguments %{c_compiler} -c -Werror -I. %t/source_2.c

# RUN: cd %T; %{bear} --verbose --output %t.json --config %t/config.json --force-wrapper -- %{shell} %t/build.sh
# RUN: assert_compilation %t.json count -eq 2
# RUN: assert_compilation %t.json contains -file %t/source_1.c -directory %T -arguments %{c_compiler} -c -I. %t/source_1.c
# RUN: assert_compilation %t.json contains -file %t/source_2.c -directory %T -arguments %{c_compiler} -c -Werror -I. %t/source_2.c

TEST=$1

mkdir $TEST
touch $TEST/source_1.c;
touch $TEST/source_2.c;

cat > "$TEST/build.sh" << EOF
#!/usr/bin/env sh

\$CC -c "$TEST/source_1.c" -Wall;
\$CC -c "$TEST/source_2.c" -Werror;
EOF


cat > "$TEST/config.json" << EOF
{
  "citnames": {
    "output": {
      "content": {
        "include_only_existing_source": true
      },
      "format": {
        "command_as_array": true,
        "drop_output_field": true
      }
    },
    "compilation": {
      "compilers_to_recognize": [
        {
          "executable": "$CC",
          "flags_to_add": ["-I."],
          "flags_to_remove": ["-Wall"]
        }
      ]
    }
  }
}
EOF
