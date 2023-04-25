#!/usr/bin/env sh

# REQUIRES: shell
# RUN: cd %T; %{bear} --verbose --output-compile %t.json --output-link %t_link.json -- %{shell} %s
# RUN: assert_compilation %t.json count -eq 1
# RUN: assert_compilation %t.json contains -file %T/without_link_flag.c -directory %T -arguments %{c_compiler} -c -o without_link_flag.c.o without_link_flag.c


echo "int main() { return 0; }" > without_link_flag.c

$CC -o without_link_flag without_link_flag.c
