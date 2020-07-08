#!/usr/bin/env sh

# REQUIRES: shell, dynamic-shell
# RUN: %{shell} %s > %t.orig.stdout
# RUN: %{intercept} --force-wrapper --output %t.json -- %{shell} %s > %t.fwd.stdout
# RUN: diff %t.orig.stdout %t.fwd.stdout

$ECHO "Lorem ipsum dolor sit amet, consectetur adipiscing elit,"
$ECHO "sed do eiusmod tempor incididunt ut labore et dolore magna aliqua."
