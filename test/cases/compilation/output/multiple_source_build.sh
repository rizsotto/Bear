#!/usr/bin/env sh

# REQUIRES: shell
# RUN: cd %T; %{bear} -vvvv --cdb %t.json -- %{shell} %s
# RUN: assert_compilation %t.json count -eq 3
# RUN: assert_compilation %t.json contains -file %T/multiple_source_build_1.c -directory %T -arguments %{c_compiler} -c -o multiple_source_build multiple_source_build_1.c
# RUN: assert_compilation %t.json contains -file %T/multiple_source_build_2.c -directory %T -arguments %{c_compiler} -c -o multiple_source_build multiple_source_build_2.c
# RUN: assert_compilation %t.json contains -file %T/multiple_source_build_3.c -directory %T -arguments %{c_compiler} -c -o multiple_source_build multiple_source_build_3.c

echo "int foo() { return 1; }" > multiple_source_build_1.c
echo "int bar() { return 1; }" > multiple_source_build_2.c
echo "int main() { return 0; }" > multiple_source_build_3.c

$CC -o multiple_source_build multiple_source_build_1.c multiple_source_build_2.c multiple_source_build_3.c
