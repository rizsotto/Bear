add_dependencies(check ear)

message(STATUS "Looking for lit")
find_program(LIT_EXECUTABLE
  NAMES lit
  PATHS ENV LIT_PATH)
if (LIT_EXECUTABLE)
  set(LIT_FOUND 1)
  message(STATUS "Looking for lit -- found")
endif()

if(LIT_FOUND)
  set(EAR_LIB_FILE ${CMAKE_SHARED_LIBRARY_PREFIX}ear${CMAKE_SHARED_LIBRARY_SUFFIX})
  set(BEAR_EXE ${CMAKE_BINARY_DIR}/source/bear/bear)
  set(PEAR_EXE ${CMAKE_BINARY_DIR}/source/pear/pear)
  set(EAR_LIB ${CMAKE_BINARY_DIR}/source/libear/${EAR_LIB_FILE})

  add_test(NAME func_test
    COMMAND lit -DEAR_EXE=${BEAR_EXE} -DPEAR_EXE=${PEAR_EXE} -DEAR_LIB=${EAR_LIB} -v ${CMAKE_CURRENT_SOURCE_DIR})
endif()
