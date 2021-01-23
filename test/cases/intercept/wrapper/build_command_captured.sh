#!/usr/bin/env sh

# REQUIRES: shell
# RUN: %{intercept} --force-wrapper --verbose --output %t.sqlite3 -- env
# RUN: assert_intercepted %t.sqlite3 count -eq 0
