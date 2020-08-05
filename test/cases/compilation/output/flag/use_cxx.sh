#!/usr/bin/env sh

# REQUIRES: shell

# RUN: cd %T; %{bear} -vvvv --cdb %t.known.json -- %{shell} %s %{true}
# RUN: assert_compilation %t.known.json count -eq 2
# RUN: assert_compilation %t.known.json contains -file %T/use_cxx_1.cc -directory %T -arguments %{cxx_compiler} -c -o use_cxx_1.o use_cxx_1.cc
# RUN: assert_compilation %t.known.json contains -file %T/use_cxx_2.cc -directory %T -arguments %{cxx_compiler} -c -o use_cxx_2.o use_cxx_2.cc

# RUN: cd %T; env CXX=%{true} %{bear} -vvvv --cdb %t.all.json -- %{shell} %s %{true}
# RUN: assert_compilation %t.all.json count -eq 4
# RUN: assert_compilation %t.all.json contains -file %T/use_cxx_1.cc -directory %T -arguments %{true} -c -o use_cxx_1.o use_cxx_1.cc
# RUN: assert_compilation %t.all.json contains -file %T/use_cxx_2.cc -directory %T -arguments %{true} -c -o use_cxx_2.o use_cxx_2.cc
# RUN: assert_compilation %t.all.json contains -file %T/use_cxx_3.cc -directory %T -arguments %{true} -c -o use_cxx_3.o use_cxx_3.cc
# RUN: assert_compilation %t.all.json contains -file %T/use_cxx_4.cc -directory %T -arguments %{true} -c -o use_cxx_4.o use_cxx_4.cc

touch use_cxx_1.cc use_cxx_2.cc use_cxx_3.cc use_cxx_4.cc

$CXX -c -o use_cxx_1.o use_cxx_1.cc;
$CXX -c -o use_cxx_2.o use_cxx_2.cc;
$1 -c -o use_cxx_3.o use_cxx_3.cc;
$1 -c -o use_cxx_4.o use_cxx_4.cc;
