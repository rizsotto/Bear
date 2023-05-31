#!/usr/bin/env sh

# REQUIRES: shell

# RUN: printf "int foo() { return 1; }" > %T/libwith_static_lib.c
# RUN: gcc -c %T/libwith_static_lib.c -o %T/libwith_static_lib.o
# RUN: ar -q -c %T/libwith_static_lib.a %T/libwith_static_lib.o

# RUN: cd %T; %{bear} --verbose --with-link --output-compile %t.json --output-link %t_link.json -- %{shell} %s
# RUN: assert_compilation %t.json count -eq 1
# RUN: assert_compilation %t.json contains -file %T/with_static_lib.c -files %T/libwith_static_lib.a -directory %T -arguments %{c_compiler} -c -L. -lwith_static_lib -o with_static_lib.c.o with_static_lib.c
# RUN: assert_compilation %t_link.json count -eq 1
# RUN: assert_compilation %t_link.json contains -files %T/libwith_static_lib.a %T/with_static_lib.c.o -directory %T -arguments %{c_compiler} -L. -lwith_static_lib with_static_lib.c.o -o with_static_lib

echo "int main() { return 0; }" > with_static_lib.c

$CC -o with_static_lib -L. -lwith_static_lib with_static_lib.c
