enable_testing()
set(CMAKE_CTEST_COMMAND ctest -V)
add_custom_target(check COMMAND ${CMAKE_CTEST_COMMAND})

add_dependencies(check ear)

add_subdirectory(end-to-end)


message(STATUS "Looking for lit")
find_program(LIT_EXECUTABLE
  NAMES lit
  PATHS ENV LIT_PATH)
if (LIT_EXECUTABLE)
  set(LIT_FOUND 1)
  message(STATUS "Looking for lit -- found")
endif()

if(LIT_FOUND)
  set(EAR_EXE ${CMAKE_BINARY_DIR}/bear/bear)
  set(EAR_LIB ${CMAKE_BINARY_DIR}/libear/${EAR_LIB_FILE})

  add_test(NAME func_test
    COMMAND lit -DEAR_EXE=${EAR_EXE} -DEAR_LIB=${EAR_LIB} -v ${CMAKE_CURRENT_SOURCE_DIR})
endif()