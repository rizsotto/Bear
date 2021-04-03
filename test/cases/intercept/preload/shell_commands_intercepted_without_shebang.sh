#!/usr/bin/env sh

# REQUIRES: preload, shell, dynamic-shell
# RUN: %{shell} %s > %t.sh
# RUN: chmod +x %t.sh
# RUN: %{intercept} --verbose --output %t.sqlite3 -- %t.sh
# RUN: %{events_db} dump --path %t.sqlite3 --output %t.json
# RUN: assert_intercepted %t.json count -ge 4
# RUN: assert_intercepted %t.json contains -program %{true}
# RUN: assert_intercepted %t.json contains -program %t.sh -arguments %t.sh

cat << EOF
$TRUE
$TRUE
$TRUE
EOF
