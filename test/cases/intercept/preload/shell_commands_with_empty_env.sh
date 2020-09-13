#!/usr/bin/env sh

# REQUIRES: preload
# RUN: %{intercept} --verbose --output %t.json -- env - %{shell} %s %{true}
# RUN: assert_intercepted %t.json count -ge 5
# RUN: assert_intercepted %t.json contains -program %{true}
# RUN: assert_intercepted %t.json contains -program %{shell} -arguments %{shell} %s %{true}

$1;
$1;
$1;
