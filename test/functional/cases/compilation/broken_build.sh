#!/usr/bin/env sh

# REQUIRES: preload, shell, dynamic-shell
# RUN: cp %{input_dir}/compile_error.c %T/broken.c
# RUN: cd %T; %{bear} -vvvv --cdb %t.json -- %{shell} %s || true
# RUN: assert_compilation %t.json count -eq 1
# RUN: assert_compilation %t.json contains -file broken.c -directory %T -arguments %{c_compiler} -c broken.c

$CC -c broken.c;
