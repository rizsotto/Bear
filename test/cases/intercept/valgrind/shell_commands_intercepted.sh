#!/usr/bin/env sh

# REQUIRES: preload, shell, dynamic-shell, valgrind
# RUN: %{valgrind} --tool=memcheck --trace-children=yes %{intercept} --verbose --output %t.json -- %{shell} %s
# RUN: assert_intercepted %t.json count -ge 4

$TRUE;
$TRUE;
$TRUE;
