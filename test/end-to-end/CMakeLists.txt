# This file is distributed under MIT-LICENSE. See COPYING for details.

message(STATUS "Looking for scons")
find_program(SCONS_EXECUTABLE
  NAMES scons
  PATHS ENV SCONS_PATH)
if (SCONS_EXECUTABLE)
  set(SCONS_FOUND 1)
  message(STATUS "Looking for scons -- found")
endif()

message(STATUS "Looking for qmake")
find_program(QMAKE_EXECUTABLE
  NAMES qmake qmake-qt4
  PATHS ENV QMAKE_PATH)
if (QMAKE_EXECUTABLE)
  set(QMAKE_FOUND 1)
  message(STATUS "Looking for qmake -- found")
endif()


set(BEAR_EXE ${CMAKE_CURRENT_BINARY_DIR}/../../src/bear)
set(BEAR_LIB ${CMAKE_CURRENT_BINARY_DIR}/../../src/libear.so)

if(true)
  add_custom_target(run_shell_test
    COMMAND ${BEAR_EXE} -b ${BEAR_LIB} -o shell.json -- ${CMAKE_CURRENT_SOURCE_DIR}/compile.sh ${CMAKE_CURRENT_SOURCE_DIR}
    COMMAND test `${CMAKE_CURRENT_SOURCE_DIR}/validate_and_count.py < shell.json` -eq 4
    COMMENT "Running module tests with shell")

  add_dependencies(run_shell_test ear)
  add_dependencies(run_shell_test bear)

  add_test(NAME run_shell_test
    COMMAND make run_shell_test)
endif()

if(SCONS_FOUND)
  add_custom_target(run_scons_test
    COMMAND ${BEAR_EXE} -b ${BEAR_LIB} -o scons.json -- ${SCONS_EXECUTABLE} -Y ${CMAKE_CURRENT_SOURCE_DIR}
    COMMAND ${SCONS_EXECUTABLE} -Y ${CMAKE_CURRENT_SOURCE_DIR} -c
    COMMAND test `${CMAKE_CURRENT_SOURCE_DIR}/validate_and_count.py < scons.json` -eq 4
    COMMENT "Running module tests with scons")

  add_dependencies(run_scons_test ear)
  add_dependencies(run_scons_test bear)

  add_test(NAME run_scons_test
    COMMAND make run_scons_test)
endif()

if(QMAKE_FOUND)
  add_custom_target(run_qmake_test
    COMMAND ${QMAKE_EXECUTABLE} ${CMAKE_CURRENT_SOURCE_DIR}/qmake.pro
    COMMAND ${BEAR_EXE} -b ${BEAR_LIB} -o qmake.json -- make
    COMMAND make distclean
    COMMAND test `${CMAKE_CURRENT_SOURCE_DIR}/validate_and_count.py < qmake.json` -eq 4
    COMMENT "Running module tests with qmake")

  add_dependencies(run_qmake_test ear)
  add_dependencies(run_qmake_test bear)

  add_test(NAME run_qmake_test
    COMMAND make run_qmake_test)
endif()
