message(STATUS "Looking for lit")
find_program(LIT_EXECUTABLE
        NAMES lit
        PATHS ENV LIT_PATH)
if (LIT_EXECUTABLE)
    set(LIT_FOUND 1)
    message(STATUS "Looking for lit -- found")
endif ()

if (LIT_FOUND)
    set(EXEC_LIB_FILE ${CMAKE_SHARED_LIBRARY_PREFIX}exec${CMAKE_SHARED_LIBRARY_SUFFIX})
    set(EXEC_LIB ${CMAKE_BINARY_DIR}/source/intercept-library/library/${EXEC_LIB_FILE})
    set(INTERCEPT_EXE ${CMAKE_BINARY_DIR}/source/intercept-library/executable/er)
    set(INTERCEPT_CC_EXE ${CMAKE_BINARY_DIR}/source/intercept-wrapper/intercept-cc/intercept-cc)
    set(INTERCEPT_CXX_EXE ${CMAKE_BINARY_DIR}/source/intercept-wrapper/intercept-c++/intercept-c++)
    set(BEAR_EXE ${CMAKE_BINARY_DIR}/source/bear/source/bear)

    add_test(NAME func_test
            COMMAND lit -DBEAR_EXE=${BEAR_EXE} -DINTERCEPT_EXE=${INTERCEPT_EXE} -DEXEC_LIB=${EXEC_LIB} -v ${CMAKE_CURRENT_SOURCE_DIR})
endif ()
