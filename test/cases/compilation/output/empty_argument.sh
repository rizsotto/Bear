#!/usr/bin/env sh

# REQUIRES: shell
# RUN: cd %T; %{bear} -vvvv --cdb %t.json -- %{shell} %s
# RUN: assert_compilation %t.json count -ge 2
# RUN: assert_compilation %t.json contains -file empty_argument_1.c -directory %T
# RUN: assert_compilation %t.json contains -file empty_argument_2.c -directory %T

touch empty_argument_1.c empty_argument_2.c

# empty argument for a command
echo "" "";

# empty argument for a compiler
$CC -c -o empty_argument_1.o empty_argument_1.c "" || true;
$CC -c -o empty_argument_2.o "" empty_argument_2.c || true;
