#!/usr/bin/env sh

# REQUIRES: shell
# RUN: cd %T; %{bear} --verbose --output %t.json -- %{shell} %s
# RUN: assert_compilation %t.json count -eq 2
# RUN: assert_compilation %t.json contains -file %T/flags_filtered_preproc_1.c -directory %T -arguments %{c_compiler} -c -MD  -MF flags_filtered_preproc_1.d -o flags_filtered_preproc_1.o flags_filtered_preproc_1.c
# RUN: assert_compilation %t.json contains -file %T/flags_filtered_preproc_2.c -directory %T -arguments %{c_compiler} -c -MMD -MF flags_filtered_preproc_2.d -o flags_filtered_preproc_2.o flags_filtered_preproc_2.c

touch flags_filtered_preproc_1.c flags_filtered_preproc_2.c

# these shall not be in the output
$CC -c -M  -o flags_filtered_preproc_1.d flags_filtered_preproc_1.c;
$CC -c -MM -o flags_filtered_preproc_2.d flags_filtered_preproc_2.c;

# these shall be in the output
$CC -c -MD  -MF flags_filtered_preproc_1.d -o flags_filtered_preproc_1.o flags_filtered_preproc_1.c;
$CC -c -MMD -MF flags_filtered_preproc_2.d -o flags_filtered_preproc_2.o flags_filtered_preproc_2.c;
