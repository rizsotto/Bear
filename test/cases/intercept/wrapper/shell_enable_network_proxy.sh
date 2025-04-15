#!/usr/bin/env sh

# REQUIRES: shell, cc

# If an unreachable or an invalid http proxy is set, an error is returned if '--enable-network-proxy' is set
#
# RUN: env http_proxy=http://localhost:9999 %{intercept} --enable-network-proxy --output %t.json -- %{shell} %s 2> %t.stderr
# RUN: grep "failed to connect to all addresses" %t.stderr
# RUN: assert_intercepted %t.json count -eq 0

# If no http proxy is set, it should run correctly even with '--enable-network-proxy'
#
# RUN: %{intercept} --enable-network-proxy --output %t.json -- %{shell} %s
# RUN: assert_intercepted %t.json count -eq 1
# RUN: assert_intercepted %t.json contains -program %{c_compiler} -arguments %{c_compiler} -c shell_enable_network_proxy.c -o shell_enable_network_proxy.o

# By default, even with an invalid http proxy set, it should run correctly
#
# RUN: env http_proxy=http://localhost:9999 %{intercept} --output %t.json -- %{shell}
# RUN: assert_intercepted %t.json count -eq 1

touch shell_enable_network_proxy.c

$CC -c shell_enable_network_proxy.c -o shell_enable_network_proxy.o

