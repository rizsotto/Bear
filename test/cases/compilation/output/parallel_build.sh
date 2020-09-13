#!/usr/bin/env sh

# REQUIRES: shell
# RUN: cd %T; %{bear} --verbose --output %t.json -- %{shell} %s
# RUN: assert_compilation %t.json count -eq 4
# RUN: assert_compilation %t.json contains -file %T/parallel_build_1.c -directory %T -arguments %{c_compiler} -c -o parallel_build_1.o parallel_build_1.c
# RUN: assert_compilation %t.json contains -file %T/parallel_build_2.c -directory %T -arguments %{c_compiler} -c -o parallel_build_2.o parallel_build_2.c
# RUN: assert_compilation %t.json contains -file %T/parallel_build_3.c -directory %T -arguments %{c_compiler} -c -o parallel_build_3.o parallel_build_3.c
# RUN: assert_compilation %t.json contains -file %T/parallel_build_4.c -directory %T -arguments %{c_compiler} -c -o parallel_build_4.o parallel_build_4.c

touch parallel_build_1.c parallel_build_2.c parallel_build_3.c parallel_build_4.c

$CC -c -o parallel_build_1.o parallel_build_1.c &
$CC -c -o parallel_build_2.o parallel_build_2.c &
$CC -c -o parallel_build_3.o parallel_build_3.c &
$CC -c -o parallel_build_4.o parallel_build_4.c &

wait

true;
