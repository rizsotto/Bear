#!/usr/bin/env sh

# REQUIRES: shell
# RUN: cd %T; %{bear} -vvvv --cdb %t.json -- %{shell} %s
# RUN: assert_compilation %t.json count -eq 4
# RUN: assert_compilation %t.json contains -file %T/duplicate_entries_1.c -directory %T -arguments %{c_compiler} -c duplicate_entries_1.c -o duplicate_entries_1.o
# RUN: assert_compilation %t.json contains -file %T/duplicate_entries_2.c -directory %T -arguments %{c_compiler} -c duplicate_entries_2.c -o duplicate_entries_2.o
# RUN: assert_compilation %t.json contains -file %T/duplicate_entries_1.c -directory %T -arguments %{c_compiler} -c duplicate_entries_1.c -o duplicate_entries_3.o -D_FLAG=value
# RUN: assert_compilation %t.json contains -file %T/duplicate_entries_2.c -directory %T -arguments %{c_compiler} -c duplicate_entries_2.c -o duplicate_entries_4.o -D_FLAG=value

touch duplicate_entries_1.c duplicate_entries_2.c

$CC -c duplicate_entries_1.c -o duplicate_entries_1.o;
$CC -c duplicate_entries_2.c -o duplicate_entries_2.o;
$CC -c duplicate_entries_1.c -o duplicate_entries_3.o -D_FLAG=value;
$CC -c duplicate_entries_2.c -o duplicate_entries_4.o -D_FLAG=value;
$CC -c duplicate_entries_1.c -o duplicate_entries_1.o;
$CC -c duplicate_entries_2.c -o duplicate_entries_2.o;
$CC -c duplicate_entries_1.c -o duplicate_entries_3.o -D_FLAG=value;
$CC -c duplicate_entries_2.c -o duplicate_entries_4.o -D_FLAG=value;
