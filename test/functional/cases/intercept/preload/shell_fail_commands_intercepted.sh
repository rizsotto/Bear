#!/usr/bin/env sh

# REQUIRES: preload, shell, dynamic-shell
# RUN: %{intercept} --verbose --output %t.json -- %{shell} %s || true
# RUN: assert_intercepted %t.json count -ge 2
# RUN: assert_intercepted %t.json contains -program %{false}

$FALSE;