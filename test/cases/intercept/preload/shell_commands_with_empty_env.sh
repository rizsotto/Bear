#!/usr/bin/env sh

# REQUIRES: preload
# RUN: %{intercept} --verbose --output %t.sqlite3 -- env - %{shell} %s %{true}
# RUN: assert_intercepted %t.sqlite3 count -ge 5
# RUN: assert_intercepted %t.sqlite3 contains -program %{true}
# RUN: assert_intercepted %t.sqlite3 contains -program %{shell} -arguments %{shell} %s %{true}

$1;
$1;
$1;
