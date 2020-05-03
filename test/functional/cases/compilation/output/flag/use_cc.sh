#!/usr/bin/env sh

# REQUIRES: preload, shell, dynamic-shell

# RUN: cd %T; %{bear} -vvvv --cdb %t.known.json -- %{shell} %s %{true}
# RUN: assert_compilation %t.known.json count -eq 2
# RUN: assert_compilation %t.known.json contains -file use_cc_1.c -directory %T -arguments %{c_compiler} -c -o use_cc_1.o use_cc_1.c
# RUN: assert_compilation %t.known.json contains -file use_cc_2.c -directory %T -arguments %{c_compiler} -c -o use_cc_2.o use_cc_2.c

# RUN: cd %T; %{bear} -vvvv --cdb %t.all.json --use-cc=%{true} -- %{shell} %s %{true}
# RUN: assert_compilation %t.all.json count -eq 4
# RUN: assert_compilation %t.all.json contains -file use_cc_1.c -directory %T -arguments %{c_compiler} -c -o use_cc_1.o use_cc_1.c
# RUN: assert_compilation %t.all.json contains -file use_cc_2.c -directory %T -arguments %{c_compiler} -c -o use_cc_2.o use_cc_2.c
# RUN: assert_compilation %t.all.json contains -file use_cc_3.c -directory %T -arguments %{true} -c -o use_cc_3.o use_cc_3.c
# RUN: assert_compilation %t.all.json contains -file use_cc_4.c -directory %T -arguments %{true} -c -o use_cc_4.o use_cc_4.c

# RUN: cd %T; %{bear} -vvvv --cdb %t.only.json --use-only --use-cc=%{true} -- %{shell} %s %{true}
# RUN: assert_compilation %t.only.json count -eq 2
# RUN: assert_compilation %t.only.json contains -file use_cc_3.c -directory %T -arguments %{true} -c -o use_cc_3.o use_cc_3.c
# RUN: assert_compilation %t.only.json contains -file use_cc_4.c -directory %T -arguments %{true} -c -o use_cc_4.o use_cc_4.c

touch use_cc_1.c use_cc_2.c use_cc_3.c use_cc_4.c

$CC -c -o use_cc_1.o use_cc_1.c;
$CC -c -o use_cc_2.o use_cc_2.c;
$1 -c -o use_cc_3.o use_cc_3.c;
$1 -c -o use_cc_4.o use_cc_4.c;
