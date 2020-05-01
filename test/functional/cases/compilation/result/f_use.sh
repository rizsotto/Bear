#!/usr/bin/env sh

# REQUIRES: preload, shell, dynamic-shell

# RUN: cd %T; %{bear} -vvvv --cdb %t.known.json -- %{shell} %s %T %{true}
# RUN: assert_compilation %t.known.json count -eq 2
# RUN: assert_compilation %t.known.json contains -file f_use_1.c -directory %T -arguments %{c_compiler} -c -o f_use_1.o f_use_1.c
# RUN: assert_compilation %t.known.json contains -file f_use_2.c -directory %T -arguments %{c_compiler} -c -o f_use_2.o f_use_2.c

# RUN: cd %T; %{bear} -vvvv --cdb %t.all.json --use-cc=%{true} -- %{shell} %s %T %{true}
# RUN: assert_compilation %t.all.json count -eq 4
# RUN: assert_compilation %t.all.json contains -file f_use_1.c -directory %T -arguments %{c_compiler} -c -o f_use_1.o f_use_1.c
# RUN: assert_compilation %t.all.json contains -file f_use_2.c -directory %T -arguments %{c_compiler} -c -o f_use_2.o f_use_2.c
# RUN: assert_compilation %t.all.json contains -file f_use_3.c -directory %T -arguments %{true} -c -o f_use_3.o f_use_3.c
# RUN: assert_compilation %t.all.json contains -file f_use_4.c -directory %T -arguments %{true} -c -o f_use_4.o f_use_4.c

# RUN: cd %T; %{bear} -vvvv --cdb %t.only.json --use-only --use-cc=%{true} -- %{shell} %s %T %{true}
# RUN: assert_compilation %t.only.json count -eq 2
# RUN: assert_compilation %t.only.json contains -file f_use_3.c -directory %T -arguments %{true} -c -o f_use_3.o f_use_3.c
# RUN: assert_compilation %t.only.json contains -file f_use_4.c -directory %T -arguments %{true} -c -o f_use_4.o f_use_4.c

touch f_use_1.c f_use_2.c f_use_3.c f_use_4.c

$CC -c -o f_use_1.o f_use_1.c;
$CC -c -o f_use_2.o f_use_2.c;
$2 -c -o f_use_3.o f_use_3.c;
$2 -c -o f_use_4.o f_use_4.c;
