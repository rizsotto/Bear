#!/usr/bin/env sh

# REQUIRES: shell

# RUN: printf "int foo() { return 1; }" > %T/libsome_libs.c
# RUN: gcc -c %T/libsome_libs.c -o %T/libsome_libs.o
# RUN: ar -q -c %T/libsome_libs.a %T/libsome_libs.o

# RUN: echo "int bar() { return 2; }" > %T/libsome_libs_1.c
# RUN: gcc -c -o %T/libsome_libs_1.o -fpic %T/libsome_libs_1.c
# RUN: gcc %{some_libs_flags} %T/libsome_libs_1.o

# RUN: mkdir -p %T/other
# RUN: printf "int foo() { return 1; }" > %T/other/libsome_libs.c
# RUN: gcc -c %T/other/libsome_libs.c -o %T/other/libsome_libs.o
# RUN: ar -q -c %T/other/libsome_libs.a %T/other/libsome_libs.o

# RUN: cd %T; %{bear} --verbose --with-link --output-compile %t.json --output-link %t_link.json -- %{shell} %s
# RUN: assert_compilation %t.json count -eq 1
# RUN: assert_compilation %t.json contains -file %T/some_libs.c -files %T/libsome_libs.%{dynamic_lib_extension} -directory %T -arguments %{c_compiler} -c -L ./other -L. -lsome_libs -o some_libs.c.o some_libs.c
# RUN: assert_compilation %t_link.json count -eq 1
# RUN: assert_compilation %t_link.json contains -files %T/libsome_libs.%{dynamic_lib_extension} %T/some_libs.c.o -directory %T -arguments %{c_compiler} -L ./other -L. -lsome_libs some_libs.c.o -o some_libs

echo "int main() { return 0; }" > some_libs.c

$CC -o some_libs -L ./other -L. -lsome_libs some_libs.c
