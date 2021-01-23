#!/usr/bin/env sh

# REQUIRES: preload, shell, dynamic-shell, valgrind
# RUN: %{valgrind} --tool=memcheck --trace-children=yes %{intercept} --verbose --output %t.sqlite3 -- %{shell} %s
# RUN: assert_intercepted %t.sqlite3 count -ge 4

$TRUE;
$TRUE;
$TRUE;
