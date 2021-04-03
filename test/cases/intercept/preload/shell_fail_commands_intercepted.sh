#!/usr/bin/env sh

# REQUIRES: preload, shell, dynamic-shell
# RUN: %{intercept} --verbose --output %t.sqlite3 -- %{shell} %s || true
# RUN: %{events_db} dump --path %t.sqlite3 --output %t.json
# RUN: assert_intercepted %t.json count -ge 2
# RUN: assert_intercepted %t.json contains -program %{false}

$FALSE;