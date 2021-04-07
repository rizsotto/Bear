#!/usr/bin/env sh

# REQUIRES: preload, shell, dynamic-shell
# TODO: %{intercept} --verbose --output %t.events.db -- %{shell} %S/shell_command_with_iso8859-2.input
# TODO: %{events_db} dump --path %t.events.db --output %t.json
# TODO: %{shell} shell_command_with_iso8859-2.check %t.json
# RUN: %{true}
