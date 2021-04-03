#!/usr/bin/env sh

# REQUIRES: preload
# RUN: %{intercept} --verbose --output %t.sqlite3 -- %{true} %s
# RUN: %{events_db} dump --path %t.sqlite3 --output %t.json
# RUN: assert_intercepted %t.json count -eq 1
# RUN: assert_intercepted %t.json contains -program %{true} -arguments %{true} %s
