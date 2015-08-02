#!/bin/sh

set -o nounset
set -o errexit
set -o xtrace

c++ -c -fPIC -I"$1/lib" "$1/lib/foo.cpp" -o /dev/null
c++ -c -fPIC -I"$1/lib" "$1/lib/bar.cc" -o /dev/null

(c++ -c -I"$1/lib" -I"$1/bin" "$1/bin/./boo.c++" -o /dev/null)
(c++ -c -I"$1/lib" -I"$1/bin" "$1/bin/../bin/far.cxx" -o /dev/null)

# add noise to the compilation...
true
echo "gcc -invocation -look -like this.c"
echo "c++ -c bin/boo.cpp -o /dev/null"
