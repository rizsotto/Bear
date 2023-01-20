#!/usr/bin/env sh

# REQUIRES: shell
# RUN: %{shell} %s %t
# RUN: cd %T; %{bear} --verbose --output %t.json --config %t/config.json -- %{shell} %t/build.sh
# RUN: assert_compilation %t.json count -eq 2
# RUN: assert_compilation %t.json contains -file %t/source_1.c -directory %T -arguments %{c_compiler} -c %t/source_1.c
# RUN: assert_compilation %t.json contains -file %t/source_2.c -directory %T -arguments %{c_compiler} -c %t/source_2.c

TEST=$1
TEST_RELATIVE=$(basename $1)

mkdir -p $TEST;
touch $TEST/source_1.c;
touch $TEST/source_2.c;
mkdir -p $TEST/exclude;
touch $TEST/exclude/source_1.c;
touch $TEST/exclude/source_2.c;

cat > "$TEST/build.sh" << EOF
#!/usr/bin/env sh

\$CC -c $TEST/source_1.c;
\$CC -c $TEST/source_2.c;
\$CC -c $TEST/exclude/source_1.c;
\$CC -c $TEST/exclude/source_2.c;
EOF


cat > "$TEST/config.json" << EOF
{
  "citnames": {
    "output": {
      "content": {
        "include_only_existing_source": true,
        "paths_to_exclude": [
          "$TEST_RELATIVE/exclude"
        ]
      },
      "format": {
        "command_as_array": true,
        "drop_output_field": true
      }
    }
  }
}
EOF
