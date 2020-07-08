#!/usr/bin/env sh

# RUN: %{intercept} --force-wrapper --verbose --output %t.json -- env
# RUN: assert_intercepted %t.json count -eq 0
