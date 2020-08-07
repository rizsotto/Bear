#!/usr/bin/env sh

# REQUIRES: shell

# RUN: cd %T; %{bear} -vvvv --force-preload --cdb %t.known.json -- %{shell} %s
# RUN: assert_compilation %t.known.json count -eq 2
# RUN: assert_compilation %t.known.json contains -file %T/use_cxx_1.cc -directory %T -arguments %{cxx_compiler} -c -o use_cxx_1.o use_cxx_1.cc
# RUN: assert_compilation %t.known.json contains -file %T/use_cxx_2.cc -directory %T -arguments %{cxx_compiler} -c -o use_cxx_2.o use_cxx_2.cc

# RUN: cd %T; env CXX=%{true} %{bear} -vvvv --force-preload --cdb %t.all.json -- %{shell} %s
# RUN: assert_compilation %t.all.json count -eq 2
# RUN: assert_compilation %t.all.json contains -file %T/use_cxx_1.cc -directory %T -arguments %{true} -c -o use_cxx_1.o use_cxx_1.cc
# RUN: assert_compilation %t.all.json contains -file %T/use_cxx_2.cc -directory %T -arguments %{true} -c -o use_cxx_2.o use_cxx_2.cc

# RUN: cd %T; %{bear} -vvvv --force-wrapper --cdb %t.known.json -- %{shell} %s
# RUN: assert_compilation %t.known.json count -eq 2
# RUN: assert_compilation %t.known.json contains -file %T/use_cxx_1.cc -directory %T -arguments %{cxx_compiler} -c -o use_cxx_1.o use_cxx_1.cc
# RUN: assert_compilation %t.known.json contains -file %T/use_cxx_2.cc -directory %T -arguments %{cxx_compiler} -c -o use_cxx_2.o use_cxx_2.cc

# RUN: cd %T; env CXX=%{true} %{bear} -vvvv --force-wrapper --cdb %t.all.json -- %{shell} %s
# RUN: assert_compilation %t.all.json count -eq 2
# RUN: assert_compilation %t.all.json contains -file %T/use_cxx_1.cc -directory %T -arguments %{true} -c -o use_cxx_1.o use_cxx_1.cc
# RUN: assert_compilation %t.all.json contains -file %T/use_cxx_2.cc -directory %T -arguments %{true} -c -o use_cxx_2.o use_cxx_2.cc

touch use_cxx_1.cc use_cxx_2.cc

$CXX -c -o use_cxx_1.o use_cxx_1.cc;
$CXX -c -o use_cxx_2.o use_cxx_2.cc;
