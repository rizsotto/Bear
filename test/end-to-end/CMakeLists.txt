# This file is distributed under MIT-LICENSE. See COPYING for details.

set(BEAR_EXE ${CMAKE_CURRENT_BINARY_DIR}/../../src/bin/bear)
set(BEAR_LIB ${CMAKE_CURRENT_BINARY_DIR}/../../src/lib/libear.so)

add_custom_target(run_end-to-end
  COMMAND ${BEAR_EXE} -b ${BEAR_LIB} -o scons.json -- scons -C ${CMAKE_CURRENT_SOURCE_DIR} -s
  COMMAND scons -C ${CMAKE_CURRENT_SOURCE_DIR} -c -s
  COMMAND qmake-qt4 ${CMAKE_CURRENT_SOURCE_DIR}/test.pro
  COMMAND ${BEAR_EXE} -b ${BEAR_LIB} -o qmake.json -- make
  COMMAND make distclean
  COMMENT "Running module tests")

add_dependencies(run_end-to-end ear)
add_dependencies(run_end-to-end bear)
