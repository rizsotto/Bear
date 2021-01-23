#!/usr/bin/env sh

# REQUIRES: preload, fakeroot
# RUN: %{fakeroot} whoami | xargs test 'root' =
# RUN: %{intercept} --output %t.sqlite3 -- %{fakeroot} whoami | xargs test 'root' =
# RUN: assert_intercepted %t.sqlite3 count -ge 2
# RUN: assert_intercepted %t.sqlite3 contains -arguments %{fakeroot} whoami
# RUN: assert_intercepted %t.sqlite3 contains -arguments whoami
