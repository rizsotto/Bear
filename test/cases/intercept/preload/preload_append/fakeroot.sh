#!/usr/bin/env sh

# REQUIRES: preload, fakeroot
# RUN: %{fakeroot} whoami | xargs test 'root' =
# RUN: %{intercept} --output %t.events.db -- %{fakeroot} whoami | xargs test 'root' =
# RUN: %{events_db} dump --path %t.events.db --output %t.json
# RUN: assert_intercepted %t.json count -ge 2
# RUN: assert_intercepted %t.json contains -arguments %{fakeroot} whoami
# RUN: assert_intercepted %t.json contains -arguments whoami
