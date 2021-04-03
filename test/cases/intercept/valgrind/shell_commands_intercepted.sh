#!/usr/bin/env sh

# REQUIRES: preload, shell, dynamic-shell, valgrind
# RUN: %{valgrind} --tool=memcheck --trace-children=yes %{intercept} --verbose --output %t.sqlite3 -- %{shell} %s
# RUN: %{events_db} dump --path %t.sqlite3 --output %t.json
# RUN: assert_intercepted %t.json count -ge 4

$TRUE;
$TRUE;
$TRUE;
