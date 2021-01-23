#!/usr/bin/env sh

# REQUIRES: preload, shell, dynamic-shell
# TODO: %{intercept} --verbose --output %t.sqlite3 -- %{shell} %S/shell_command_with_iso8859-2.input
# TODO: %{shell} shell_command_with_iso8859-2.check %t.sqlite3
# RUN: %{true}
