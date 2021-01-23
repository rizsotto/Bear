#!/usr/bin/env sh

# REQUIRES: preload, shell, dynamic-shell
# RUN: %{shell} %s > %t.sh
# RUN: chmod +x %t.sh
# RUN: %{intercept} --verbose --output %t.sqlite3 -- %t.sh
# RUN: assert_intercepted %t.sqlite3 count -ge 4
# RUN: assert_intercepted %t.sqlite3 contains -program %{true}
# RUN: assert_intercepted %t.sqlite3 contains -program %t.sh -arguments %t.sh

cat << EOF
$TRUE
$TRUE
$TRUE
EOF
