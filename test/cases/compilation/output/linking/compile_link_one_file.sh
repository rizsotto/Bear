#!/usr/bin/env sh

# REQUIRES: shell
# RUN: cd %T; %{bear} --verbose --output %t.json --config %t.config.json -- %{shell} %s %t
# RUN: assert_compilation %t.json count -eq 1
# RUN: assert_compilation %t.json contains -file %T/compile_link_one_file.c -directory %T -arguments %{c_compiler} -c -o compile_link_one_file.c.o compile_link_one_file.c
# RUN: assert_compilation %t_link.json count -eq 1
# RUN: assert_compilation %t_link.json contains -files %T/compile_link_one_file.c.o -directory %T -arguments %{c_compiler} compile_link_one_file.c.o -o compile_link_one_file

cat > "$1.config.json" << EOF
{
  "linking": {
    "filename": "$1_link.json"
  }
}
EOF

echo "int main() { return 0; }" > compile_link_one_file.c

$CC compile_link_one_file.c -o compile_link_one_file
