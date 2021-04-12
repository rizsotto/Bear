add_library(sys_a OBJECT)
target_include_directories(sys_a PUBLIC include/)
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
