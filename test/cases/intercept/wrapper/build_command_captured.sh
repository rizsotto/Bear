#!/usr/bin/env sh

# REQUIRES: shell
# RUN: %{intercept} --force-wrapper --verbose --output %t.sqlite3 -- env
# RUN: %{events_db} dump --path %t.sqlite3 --output %t.json
# RUN: assert_intercepted %t.json count -eq 0
