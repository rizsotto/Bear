#!/usr/bin/env sh

# REQUIRES: shell
# RUN: cd %T; %{bear} --verbose --output-compile %t.json -- %{shell} %s
# RUN: assert_compilation %t.json count -eq 4
# RUN: assert_compilation %t.json contains -file %T/successful_build_1.c -directory %T -arguments %{c_compiler} -c -o successful_build_1.o successful_build_1.c
# RUN: assert_compilation %t.json contains -file %T/successful_build_2.c -directory %T -arguments %{c_compiler} -c -o successful_build_2.o successful_build_2.c
# RUN: assert_compilation %t.json contains -file %T/successful_build_3.c -directory %T -arguments %{cxx_compiler} -c -o successful_build_3.o successful_build_3.c
# RUN: assert_compilation %t.json contains -file %T/successful_build_4.c -directory %T -arguments %{cxx_compiler} -c -o successful_build_4.o successful_build_4.c

touch successful_build_1.c successful_build_2.c successful_build_3.c successful_build_4.c

$CC -c -o successful_build_1.o successful_build_1.c;
$CC -c -o successful_build_2.o successful_build_2.c;
$CXX -c -o successful_build_3.o successful_build_3.c;
$CXX -c -o successful_build_4.o successful_build_4.c;
