enable_testing()
set(CMAKE_CTEST_COMMAND ctest -V)
add_custom_target(check COMMAND ${CMAKE_CTEST_COMMAND})

add_dependencies(check ear)

#add_subdirectory(unit_test)
add_subdirectory(end-to-end)
add_subdirectory(exec_anatomy)
add_subdirectory(result_code)
add_subdirectory(bug74)
add_subdirectory(bug80)
add_subdirectory(bug81)
add_subdirectory(bug88)
add_subdirectory(bug94)

set(EAR_EXE ${CMAKE_BINARY_DIR}/bear/bear)
set(EAR_LIB ${CMAKE_BINARY_DIR}/libear/${EAR_LIB_FILE})

add_test(NAME func_test
  COMMAND lit -DEAR_EXE=${EAR_EXE} -DEAR_LIB=${EAR_LIB} -v ${CMAKE_CURRENT_SOURCE_DIR})
