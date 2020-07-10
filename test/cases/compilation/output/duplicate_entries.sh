#!/usr/bin/env sh

# REQUIRES: shell
# RUN: cd %T; %{bear} -vvvv --cdb %t.json -- %{shell} %s
# RUN: assert_compilation %t.json count -eq 4
# RUN: assert_compilation %t.json contains -file duplicate_entries_1.c -directory %T -arguments %{c_compiler} -c -o duplicate_entries_1.o duplicate_entries_1.c
# RUN: assert_compilation %t.json contains -file duplicate_entries_2.c -directory %T -arguments %{c_compiler} -c -o duplicate_entries_2.o duplicate_entries_2.c
# RUN: assert_compilation %t.json contains -file duplicate_entries_1.c -directory %T -arguments %{c_compiler} -c -D_FLAG=value -o duplicate_entries_1.o duplicate_entries_1.c
# RUN: assert_compilation %t.json contains -file duplicate_entries_2.c -directory %T -arguments %{c_compiler} -c -D_FLAG=value -o duplicate_entries_2.o duplicate_entries_2.c

touch duplicate_entries_1.c duplicate_entries_2.c

$CC -c -o duplicate_entries_1.o duplicate_entries_1.c;
$CC -c -o duplicate_entries_1.o duplicate_entries_1.c;
$CC -c -D_FLAG=value -o duplicate_entries_1.o duplicate_entries_1.c;
$CC -c -D_FLAG=value -o duplicate_entries_1.o duplicate_entries_1.c;
$CC -c -o duplicate_entries_2.o duplicate_entries_2.c;
$CC -c -o duplicate_entries_2.o duplicate_entries_2.c;
$CC -c -D_FLAG=value -o duplicate_entries_2.o duplicate_entries_2.c;
$CC -c -D_FLAG=value -o duplicate_entries_2.o duplicate_entries_2.c;
