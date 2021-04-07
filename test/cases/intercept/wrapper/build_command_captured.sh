#!/usr/bin/env sh

# REQUIRES: shell
# RUN: %{intercept} --force-wrapper --verbose --output %t.events.db -- env
# RUN: %{events_db} dump --path %t.events.db --output %t.json
# RUN: assert_intercepted %t.json count -eq 0
