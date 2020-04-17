cmake_minimum_required(VERSION 3.2)

project(Bear-FunctionalTest)

enable_testing()

message(STATUS "Looking for lit")
find_program(LIT_EXECUTABLE
        NAMES lit
        PATHS ENV LIT_PATH)

if (LIT_EXECUTABLE)
    message(STATUS "Looking for lit -- found")

    set(BEAR_EXE "${_BEAR_BIN_DIR}/bear")
    set(INTERCEPT_EXE "${_BEAR_BIN_DIR}/intercept")
    set(ER_EXE "${_BEAR_LIBEXEC_DIR}/er")
    set(EXEC_LIB_FILE "${CMAKE_SHARED_LIBRARY_PREFIX}exec${CMAKE_SHARED_LIBRARY_SUFFIX}")
    set(EXEC_LIB "${_BEAR_LIB_DIR}/${EXEC_LIB_FILE}")

    add_test(NAME bear::func_test
            COMMAND ${LIT_EXECUTABLE} -DBEAR_EXE=${BEAR_EXE} -DINTERCEPT_EXE=${INTERCEPT_EXE} -DER_EXE=${ER_EXE} -DEXEC_LIB=${EXEC_LIB} -v ${CMAKE_CURRENT_SOURCE_DIR})
endif ()
