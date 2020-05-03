#!/usr/bin/env sh

# REQUIRES: preload, shell, dynamic-shell, cuda
# RUN: cd %T; %{bear} -vvvv --cdb %t.json -- %{shell} %s %{cuda}
# RUN: assert_compilation %t.json count -eq 2
# RUN: assert_compilation %t.json contains -file successful_build_1.cu -directory %T
# RUN: assert_compilation %t.json contains -file successful_build_2.cu -directory %T

touch successful_build_1.cu successful_build_2.cu

$1 -c -o successful_build_1.o successful_build_1.cu;
$1 -c -o successful_build_2.o successful_build_2.cu;
