#!/usr/bin/env sh

# REQUIRES: shell
# RUN: cd %T; %{bear} --verbose --output-compile %t.json -- %{shell} %s
# RUN: assert_compilation %t.json count -eq 0

touch empty_argument_1.c empty_argument_2.c

# empty argument for a command
$ECHO "" "";

# empty argument for a compiler
$CC -c -o empty_argument_1.o empty_argument_1.c "" || true;
$CC -c -o empty_argument_2.o "" empty_argument_2.c || true;
