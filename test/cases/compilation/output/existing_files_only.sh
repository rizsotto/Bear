#!/usr/bin/env sh

# REQUIRES: shell

# RUN: cd %T; %{bear} -vvvv --cdb %t.json -- %{shell} %s -build
# RUN: assert_compilation %t.json count -ge 2
# RUN: assert_compilation %t.json contains -file exists/src/source_1.c -directory %T -arguments %{c_compiler} -c -o exists/src/source_1.o exists/src/source_1.c
# RUN: assert_compilation %t.json contains -file exists/src/source_2.c -directory %T -arguments %{c_compiler} -c -o exists/src/source_2.o exists/src/source_2.c

mkdir -p exists exists/config
touch exists/config/source_1.c exists/config/source_2.c
$CC -c -o exists/config/source_1.o exists/config/source_1.c
$CC -c -o exists/config/source_2.o exists/config/source_2.c
rm exists/config/source_1.c exists/config/source_2.c

mkdir -p exists exists/src
touch exists/src/source_1.c exists/src/source_2.c
$CC -c -o exists/src/source_1.o exists/src/source_1.c
$CC -c -o exists/src/source_2.o exists/src/source_2.c
