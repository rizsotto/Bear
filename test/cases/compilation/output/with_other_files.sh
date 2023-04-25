#!/usr/bin/env sh

# REQUIRES: shell
# RUN: cd %T; %{bear} --verbose --output-compile %t.json -- %{shell} %s
# RUN: assert_compilation %t.json count -eq 1
# RUN: assert_compilation %t.json contains -file %T/with_other_files.c -files %T/lib.o -directory %T -arguments %{c_compiler} -c lib.o -o with_other_files.o with_other_files.c

touch with_other_files.c lib.o

$CC -c lib.o with_other_files.c -o with_other_files.o;
