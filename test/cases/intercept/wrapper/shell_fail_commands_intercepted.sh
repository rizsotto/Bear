#!/usr/bin/env sh

# REQUIRES: shell
# RUN: cd %T; %{intercept} --force-wrapper --verbose --output %t.json -- %{shell} %s || true
# RUN: assert_intercepted %t.json count -ge 1
# RUN: assert_intercepted %t.json contains -arguments %{cxx_compiler} -c not_exists.cc -o not_exists.o

$CXX -c not_exists.cc -o not_exists.o;
