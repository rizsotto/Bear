#!/usr/bin/env sh

# REQUIRES: shell

# RUN: cd %T; %{bear} --verbose --force-preload --output %t.known.json -- %{shell} %s
# RUN: assert_compilation %t.known.json count -eq 2
# RUN: assert_compilation %t.known.json contains -file %T/use_cc_1.c -directory %T -arguments %{c_compiler} -c -o use_cc_1.o use_cc_1.c
# RUN: assert_compilation %t.known.json contains -file %T/use_cc_2.c -directory %T -arguments %{c_compiler} -c -o use_cc_2.o use_cc_2.c

# RUN: cd %T; env CC=%{true} %{bear} --verbose --force-preload --output %t.all.json -- %{shell} %s
# RUN: assert_compilation %t.all.json count -eq 2
# RUN: assert_compilation %t.all.json contains -file %T/use_cc_1.c -directory %T -arguments %{true} -c -o use_cc_1.o use_cc_1.c
# RUN: assert_compilation %t.all.json contains -file %T/use_cc_2.c -directory %T -arguments %{true} -c -o use_cc_2.o use_cc_2.c

# RUN: cd %T; %{bear} --verbose --force-wrapper --output %t.known.json -- %{shell} %s
# RUN: assert_compilation %t.known.json count -eq 2
# RUN: assert_compilation %t.known.json contains -file %T/use_cc_1.c -directory %T -arguments %{c_compiler} -c -o use_cc_1.o use_cc_1.c
# RUN: assert_compilation %t.known.json contains -file %T/use_cc_2.c -directory %T -arguments %{c_compiler} -c -o use_cc_2.o use_cc_2.c

# RUN: cd %T; env CC=%{true} %{bear} --verbose --force-wrapper --output %t.all.json -- %{shell} %s
# RUN: assert_compilation %t.all.json count -eq 2
# RUN: assert_compilation %t.all.json contains -file %T/use_cc_1.c -directory %T -arguments %{true} -c -o use_cc_1.o use_cc_1.c
# RUN: assert_compilation %t.all.json contains -file %T/use_cc_2.c -directory %T -arguments %{true} -c -o use_cc_2.o use_cc_2.c

touch use_cc_1.c use_cc_2.c

$CC -c -o use_cc_1.o use_cc_1.c;
$CC -c -o use_cc_2.o use_cc_2.c;
