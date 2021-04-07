#!/usr/bin/env sh

# REQUIRES: preload
# RUN: %{intercept} --verbose --output %t.events.db -- %{true} %s
# RUN: %{events_db} dump --path %t.events.db --output %t.json
# RUN: assert_intercepted %t.json count -eq 1
# RUN: assert_intercepted %t.json contains -program %{true} -arguments %{true} %s
