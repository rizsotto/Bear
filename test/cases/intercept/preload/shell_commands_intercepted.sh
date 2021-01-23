#!/usr/bin/env sh

# REQUIRES: preload, shell, dynamic-shell
# RUN: %{intercept} --verbose --output %t.sqlite3 -- %{shell} %s
# RUN: assert_intercepted %t.sqlite3 count -ge 4
# RUN: assert_intercepted %t.sqlite3 contains -program %{true}
# RUN: assert_intercepted %t.sqlite3 contains -program %{shell} -arguments %{shell} %s

$TRUE;
$TRUE;
$TRUE;
