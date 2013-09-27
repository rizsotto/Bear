#!/bin/sh

set -o nounset
set -o errexit
set -o xtrace

g++ -c -fPIC -I"$1/lib" "$1/lib/foo.cpp" -o /dev/null
g++ -c -fPIC -I"$1/lib" "$1/lib/bar.cc" -o /dev/null

g++ -M -I"$1/lib" "$1/lib/foo.cpp" -o /dev/null
g++ -MM -I"$1/lib" "$1/lib/foo.cpp" -o /dev/null
g++ -MM -MG -I"$1/lib" "$1/lib/bar.cc" -o /dev/null
g++ -I"$1/lib" "$1/lib/bar.cc" -M -o /dev/null

# add noise to the compilation...
true
echo "gcc -invocation -look -like this.c"

(g++ -c -I"$1/lib" -I"$1/bin" "$1/bin/boo.c++" -o /dev/null)
(g++ -c -I"$1/lib" -I"$1/bin" "$1/bin/far.cxx" -o /dev/null)

# add noise to the compilation...
echo "g++ -c bin/boo.cpp -o /dev/null"
