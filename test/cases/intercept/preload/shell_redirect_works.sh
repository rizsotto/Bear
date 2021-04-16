#!/usr/bin/env sh

# REQUIRES: preload, shell, dynamic-shell
# RUN: %{intercept} --output %t.events.db -- %{shell} -c "%{echo} hi | %{shell} %s"
# RUN: %{events_db} dump --path %t.events.db --output %t.json
# RUN: assert_intercepted %t.json count -eq 4
# RUN: assert_intercepted %t.json contains -program %{shell}
# RUN: assert_intercepted %t.json contains -program %{echo} -arguments %{echo} "hi"
# RUN: assert_intercepted %t.json contains -program %{echo} -arguments %{echo} "hi there"

while read line
do
  $ECHO "$line there"
done
