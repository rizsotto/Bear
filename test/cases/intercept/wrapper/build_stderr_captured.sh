#!/usr/bin/env sh

# REQUIRES: shell
# RUN: %{shell} %s 2> %t.orig.stderr
# RUN: %{intercept} --force-wrapper --output %t.sqlite3 -- %{shell} %s 2> %t.fwd.stderr
# RUN: diff %t.orig.stderr %t.fwd.stderr

>&2 $ECHO "Lorem ipsum dolor sit amet, consectetur adipiscing elit,"
>&2 $ECHO "sed do eiusmod tempor incididunt ut labore et dolore magna aliqua."
