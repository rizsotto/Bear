#!/usr/bin/env sh

# REQUIRES: shell, dynamic-shell

# RUN: cd %T; %{bear} -vvvv --cdb %t.all.json -- %{shell} %s
# RUN: assert_compilation %t.all.json count -eq 4
# RUN: assert_compilation %t.all.json contains -file exclude/src/source_1.c -directory %T -arguments %{c_compiler} -c -o exclude/src/source_1.o exclude/src/source_1.c
# RUN: assert_compilation %t.all.json contains -file exclude/src/source_2.c -directory %T -arguments %{c_compiler} -c -o exclude/src/source_2.o exclude/src/source_2.c
# RUN: assert_compilation %t.all.json contains -file exclude/test/source_1.c -directory %T -arguments %{c_compiler} -c -o exclude/test/source_1.o exclude/test/source_1.c
# RUN: assert_compilation %t.all.json contains -file exclude/test/source_2.c -directory %T -arguments %{c_compiler} -c -o exclude/test/source_2.o exclude/test/source_2.c

# RUN: cd %T; %{bear} -vvvv --cdb %t.ex.json --include exclude --exclude exclude/test -- %{shell} %s
# RUN: assert_compilation %t.ex.json count -eq 2
# RUN: assert_compilation %t.ex.json contains -file exclude/src/source_1.c -directory %T -arguments %{c_compiler} -c -o exclude/src/source_1.o exclude/src/source_1.c
# RUN: assert_compilation %t.ex.json contains -file exclude/src/source_2.c -directory %T -arguments %{c_compiler} -c -o exclude/src/source_2.o exclude/src/source_2.c

mkdir -p exclude exclude/src exclude/test
touch exclude/src/source_1.c exclude/src/source_2.c
touch exclude/test/source_1.c exclude/test/source_2.c

$CC -c -o exclude/src/source_1.o exclude/src/source_1.c
$CC -c -o exclude/src/source_2.o exclude/src/source_2.c
$CC -c -o exclude/test/source_1.o exclude/test/source_1.c
$CC -c -o exclude/test/source_2.o exclude/test/source_2.c
