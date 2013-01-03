# This file is distributed under MIT-LICENSE. See COPYING for details.

enable_testing()
set(CMAKE_CTEST_COMMAND ctest -V)
add_custom_target(check COMMAND ${CMAKE_CTEST_COMMAND})

add_subdirectory(unit_test)
add_subdirectory(end-to-end)
add_subdirectory(exec_anatomy)
