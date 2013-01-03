# This file is distributed under MIT-LICENSE. See COPYING for details.

add_executable(unit_test main.c)
add_definitions(-D_GNU_SOURCE)
target_link_libraries(unit_test common)
include_directories(../../src/common)

add_test(unit_test ./unit_test)
add_dependencies(check unit_test)
