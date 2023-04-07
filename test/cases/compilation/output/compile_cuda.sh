#!/usr/bin/env sh

# REQUIRES: shell, cuda
# RUN: cd %T; env CC=%{cuda} %{bear} --verbose --output-compile %t.json -- %{shell} %s
# RUN: assert_compilation %t.json count -eq 2
# RUN: assert_compilation %t.json contains -file %T/successful_build_1.cu -directory %T
# RUN: assert_compilation %t.json contains -file %T/successful_build_2.cu -directory %T

touch successful_build_1.cu successful_build_2.cu

$CC -c -o successful_build_1.o successful_build_1.cu;
$CC -c -o successful_build_2.o successful_build_2.cu;
