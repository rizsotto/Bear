#!/usr/bin/env sh

# REQUIRES: shell, dynamic-shell
# RUN: cd %T; %{bear} -vvvv --cdb %t.json --field-output -- %{shell} %s
# RUN: assert_compilation %t.json count -eq 2
# RUN: assert_compilation %t.json contains -file f_field_output_1.c -output f_field_output_1.o -directory %T -arguments %{c_compiler} -c -o f_field_output_1.o f_field_output_1.c
# RUN: assert_compilation %t.json contains -file f_field_output_2.c -output f_field_output_2.o -directory %T -arguments %{c_compiler} -c -o f_field_output_2.o f_field_output_2.c

touch f_field_output_1.c f_field_output_2.c

$CC -c -o f_field_output_1.o f_field_output_1.c;
$CC -c -o f_field_output_2.o f_field_output_2.c;
