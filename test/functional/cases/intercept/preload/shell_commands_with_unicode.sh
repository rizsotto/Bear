#!/usr/bin/env sh

# REQUIRES: preload, shell, dynamic-shell
# RUN: %{intercept} --verbose --output %t.json -- %{shell} %s
# RUN: assert_intercepted %t.json count -ge 6
# RUN: assert_intercepted %t.json contains -program %{true}
# RUN: assert_intercepted %t.json contains -program %{echo} -arguments %{echo} "hi there \"people\""
# RUN: assert_intercepted %t.json contains -program %{echo} -arguments %{echo} "hi	again"
# RUN: assert_intercepted %t.json contains -program %{echo} -arguments %{echo} "מה שלומך?"
# RUN: assert_intercepted %t.json contains -program %{echo} -arguments %{echo} "Как дела?"
# RUN: assert_intercepted %t.json contains -program %{echo} -arguments %{echo} "[1mThis line might cause an exception in json load[0m"
# RUN: assert_intercepted %t.json contains -arguments %{shell} %s

$ECHO "hi there \"people\""
$ECHO "hi	again"
$ECHO "מה שלומך?"
$ECHO "Как дела?"
$ECHO "[1mThis line might cause an exception in json load[0m"

$TRUE
