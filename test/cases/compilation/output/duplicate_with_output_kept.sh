#!/usr/bin/env sh

# REQUIRES: shell
# RUN: cd %T; %{bear} -vvvv --cdb %t.json -- %{shell} %s
# RUN: assert_compilation %t.json count -eq 2
# RUN: assert_compilation %t.json contains -file duplicate_with_output_kept_1.c -directory %T -arguments %{c_compiler} -c -o duplicate_with_output_kept_1.o duplicate_with_output_kept_1.c
# RUN: assert_compilation %t.json contains -file duplicate_with_output_kept_2.c -directory %T -arguments %{c_compiler} -c -o duplicate_with_output_kept_2.o duplicate_with_output_kept_2.c

touch duplicate_with_output_kept_1.c duplicate_with_output_kept_2.c

$CC -c -o duplicate_with_output_kept_1.o duplicate_with_output_kept_1.c;
$CC -c -o duplicate_with_output_kept_1.obj duplicate_with_output_kept_1.c;
$CC -c duplicate_with_output_kept_1.c;
$CC -c -o duplicate_with_output_kept_2.o duplicate_with_output_kept_2.c;
$CC -c -o duplicate_with_output_kept_2.obj duplicate_with_output_kept_2.c;
$CC -c duplicate_with_output_kept_2.c;
