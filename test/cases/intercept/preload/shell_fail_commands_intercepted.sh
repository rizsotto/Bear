#!/usr/bin/env sh

# REQUIRES: preload, shell, dynamic-shell
# RUN: %{intercept} --verbose --output %t.sqlite3 -- %{shell} %s || true
# RUN: assert_intercepted %t.sqlite3 count -ge 2
# RUN: assert_intercepted %t.sqlite3 contains -program %{false}

$FALSE;