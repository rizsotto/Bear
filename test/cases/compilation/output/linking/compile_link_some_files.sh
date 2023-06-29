#!/usr/bin/env sh

# REQUIRES: shell
# RUN: cd %T; %{bear} --verbose --output %t.json --config %t.config.json -- %{shell} %s %t
# RUN: assert_compilation %t.json count -eq 2
# RUN: assert_compilation %t.json contains -file %T/compile_link_some_files_1.c -directory %T -arguments %{c_compiler} -c -o compile_link_some_files_1.c.o compile_link_some_files_1.c
# RUN: assert_compilation %t.json contains -file %T/compile_link_some_files_2.c -directory %T -arguments %{c_compiler} -c -o compile_link_some_files_2.c.o compile_link_some_files_2.c
# RUN: assert_compilation %t_link.json count -eq 1
# RUN: assert_compilation %t_link.json contains -files %T/compile_link_some_files_1.c.o %T/compile_link_some_files_2.c.o -directory %T -arguments %{c_compiler} compile_link_some_files_1.c.o compile_link_some_files_2.c.o -o compile_link_some_files

cat > "$1.config.json" << EOF
{
  "linking": {
    "filename": "$1_link.json"
  }
}
EOF

echo "int foo() { return 1; }" > compile_link_some_files_1.c
echo "int main() { return 0; }" > compile_link_some_files_2.c

$CC compile_link_some_files_1.c -o compile_link_some_files compile_link_some_files_2.c
