add_library(sys_a OBJECT)
target_include_directories(sys_a PUBLIC include/)
target_include_directories(sys_a PRIVATE ${CMAKE_CURRENT_BINARY_DIR}/include/)
target_sources(sys_a
        PRIVATE
            source/Os.cc
            source/Guard.cc
            source/Errors.cc
            source/Path.cc
            source/Process.cc
            source/Signal.cc
        INTERFACE
            $<TARGET_OBJECTS:sys_a>
        )
target_link_libraries(sys_a PUBLIC
        ${CMAKE_DL_LIBS}
        result_a
        fmt::fmt
        spdlog::spdlog)

if (ENABLE_UNIT_TESTS)
    add_executable(sys_unit_test
            test/EnvironmentTest.cc
            test/ErrorsTest.cc
            test/PathTest.cc
            )

    target_include_directories(sys_unit_test PRIVATE source/)
    target_link_libraries(sys_unit_test sys_a)
    target_link_libraries(sys_unit_test PkgConfig::GTest ${CMAKE_THREAD_LIBS_INIT})

    add_test(NAME bear::sys_unit_test COMMAND $<TARGET_FILE:sys_unit_test>)
endif ()

if (SUPPORT_PRELOAD AND NOT HAVE_GNU_LIB_NAMES_H)
    if (CMAKE_SYSTEM_NAME STREQUAL "FreeBSD")
        execute_process(
                COMMAND sh -c "ls /lib/libc.so.*"
                OUTPUT_VARIABLE LIBC_SO_PATH
                OUTPUT_STRIP_TRAILING_WHITESPACE
        )
        # cmake_path(GET LIBC_SO_PATH FILENAME LIBC_SO)
        get_filename_component(LIBC_SO ${LIBC_SO_PATH} NAME)
    elseif (CMAKE_SYSTEM_NAME STREQUAL "OpenBSD")
        execute_process(
                COMMAND sh -c "ls /usr/lib/libc.so.*"
                OUTPUT_VARIABLE LIBC_SO_PATH
                OUTPUT_STRIP_TRAILING_WHITESPACE
        )
        # cmake_path(GET LIBC_SO_PATH FILENAME LIBC_SO)
        get_filename_component(LIBC_SO ${LIBC_SO_PATH} NAME)
    elseif (HAIKU)
        set(LIBC_SO "libroot.so")
    else()
        set(LIBC_SO "libc.so")
    endif()
    message(STATUS "libc.so filename ${LIBC_SO}")
endif()

configure_file(
        include/libsys/lib-names.h.in
        include/libsys/lib-names.h
        @ONLY
)
