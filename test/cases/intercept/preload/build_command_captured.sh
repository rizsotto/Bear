#!/usr/bin/env sh

# REQUIRES: preload
# RUN: %{intercept} --verbose --output %t.sqlite3 -- %{true} %s
# RUN: assert_intercepted %t.sqlite3 count -eq 1
# RUN: assert_intercepted %t.sqlite3 contains -program %{true} -arguments %{true} %s
