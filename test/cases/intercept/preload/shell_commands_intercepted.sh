#!/usr/bin/env sh

# REQUIRES: preload, shell, dynamic-shell
# RUN: %{intercept} --verbose --output %t.events.db -- %{shell} %s
# RUN: %{events_db} dump --path %t.events.db --output %t.json
# RUN: assert_intercepted %t.json count -ge 4
# RUN: assert_intercepted %t.json contains -program %{true}
# RUN: assert_intercepted %t.json contains -program %{shell} -arguments %{shell} %s

$TRUE;
$TRUE;
$TRUE;
