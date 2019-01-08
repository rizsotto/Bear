message(STATUS "Looking for lit")
find_program(LIT_EXECUTABLE
  NAMES lit
  PATHS ENV LIT_PATH)
if (LIT_EXECUTABLE)
  set(LIT_FOUND 1)
  message(STATUS "Looking for lit -- found")
endif()

if(LIT_FOUND)
  set(EXEC_LIB_FILE ${CMAKE_SHARED_LIBRARY_PREFIX}exec${CMAKE_SHARED_LIBRARY_SUFFIX})
  set(BEAR_EXE ${CMAKE_BINARY_DIR}/source/bear/bear)
  set(INTERCEPT_EXE ${CMAKE_BINARY_DIR}/source/intercept/intercept)
  set(EXEC_LIB ${CMAKE_BINARY_DIR}/source/libexec/${EXEC_LIB_FILE})

  add_test(NAME func_test
    COMMAND lit -DBEAR_EXE=${BEAR_EXE} -DINTERCEPT_EXE=${INTERCEPT_EXE} -DEXEC_LIB=${EXEC_LIB} -v ${CMAKE_CURRENT_SOURCE_DIR})
endif()
