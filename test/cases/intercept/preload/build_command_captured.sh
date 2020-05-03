#!/usr/bin/env sh

# REQUIRES: preload
# RUN: %{intercept} --verbose --output %t.json -- %{true} %s
# RUN: assert_intercepted %t.json count -eq 1
# RUN: assert_intercepted %t.json contains -program %{true} -arguments %{true} %s
