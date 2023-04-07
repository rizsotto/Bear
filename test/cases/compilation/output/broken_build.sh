#!/usr/bin/env sh

# REQUIRES: shell
# RUN: cd %T; %{bear} --verbose --output-compile %t.json -- %{shell} %s
# RUN: assert_compilation %t.json count -ge 1
# RUN: assert_compilation %t.json contains -file %T/broken_build.c -directory %T -arguments %{c_compiler} -c -o broken_build.o broken_build.c

echo "int test() { ;" > broken_build.c

$CC -c -o broken_build.o broken_build.c || true
