cmake_minimum_required(VERSION 3.2)

project(Bear-FunctionalTest LANGUAGES NONE)

enable_testing()

message(STATUS "Looking for lit")
find_program(LIT_EXECUTABLE
        NAMES lit
        PATHS ENV LIT_PATH)

if (LIT_EXECUTABLE)
    message(STATUS "Looking for lit -- found")

    add_test(NAME bear::func_test
            COMMAND ${LIT_EXECUTABLE}
            -D_TEST_EXEC_ROOT=${CMAKE_CURRENT_BINARY_DIR}
            -D_BEAR_BIN_DIR=${_BEAR_BIN_DIR}
            -v
            ${CMAKE_CURRENT_SOURCE_DIR})
endif ()
