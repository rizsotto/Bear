#!/usr/bin/env sh

# REQUIRES: shell

# RUN: rm -f %t.json %t_link.json
# RUN: echo "[{\"arguments\": [\"gcc\", \"-c\"]}]" > %t.json

# RUN: cd %T; %{bear} --verbose --output %t.json --config %t.config.json --append -- %{shell} %s %t
# RUN: assert_compilation %t.json count -eq 2
# RUN: assert_compilation %t.json contains -file %T/source_1.c -directory %T -arguments %{c_compiler} -c -o source_1.o source_1.c
# RUN: assert_compilation %t.json contains -file %T/source_2.c -directory %T -arguments %{c_compiler} -c -o source_2.o source_2.c
# RUN: assert_compilation %t_link.json count -eq 1
# RUN: assert_compilation %t_link.json contains -files %T/source_1.o %T/source_2.o -directory %T -arguments %{c_compiler} source_1.o source_2.o

cat > "$1.config.json" << EOF
{
  "linking": {
    "filename": "$1_link.json"
  }
}
EOF

echo "int foo() { return 1; }" > source_1.c
echo "int main() { return 0; }" > source_2.c

$CC -c -o source_1.o source_1.c
$CC -c -o source_2.o source_2.c
$CC source_1.o source_2.o
