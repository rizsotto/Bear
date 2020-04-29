#!/usr/bin/env sh

# REQUIRES: preload, shell, dynamic-shell
# RUN: cp %{input_dir}/compile_error.c %T/broken_build.c
# RUN: cd %T; %{bear} -vvvv --cdb %t.json -- %{shell} %s
# RUN: assert_compilation %t.json count -eq 1
# RUN: assert_compilation %t.json contains -file broken_build.c -directory %T -arguments %{c_compiler} -c -o broken_build.o broken_build.c

$CC -c -o broken_build.o broken_build.c || true
