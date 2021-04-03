#!/usr/bin/env sh

# REQUIRES: shell
# RUN: cd %T; %{intercept} --force-wrapper --verbose --output %t.sqlite3 -- %{shell} %s || true
# RUN: %{events_db} dump --path %t.sqlite3 --output %t.json
# RUN: assert_intercepted %t.json count -ge 1
# RUN: assert_intercepted %t.json contains -program %{cxx_compiler} -arguments %{cxx_compiler} -c not_exists.cc -o not_exists.o

$CXX -c not_exists.cc -o not_exists.o;
