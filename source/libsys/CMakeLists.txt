add_library(sys_a OBJECT
        source/Os.cc
        source/Guard.cc
        source/Errors.cc
        source/Path.cc
        source/Process.cc
        source/Signal.cc
        )

target_include_directories(sys_a PUBLIC include/)
target_link_libraries(sys_a PUBLIC ${CMAKE_DL_LIBS})
target_link_libraries(sys_a PUBLIC result_a)
target_link_libraries(sys_a PRIVATE fmt::fmt)
target_link_libraries(sys_a PRIVATE spdlog::spdlog)

if (ENABLE_UNIT_TESTS)
    add_executable(sys_unit_test
            test/EnvironmentTest.cc
            test/ErrorsTest.cc
            test/PathTest.cc
            )

    target_include_directories(sys_unit_test PRIVATE source/)
    target_link_libraries(sys_unit_test sys_a)
    target_link_libraries(sys_unit_test result_a)
    target_link_libraries(sys_unit_test fmt::fmt)
    target_link_libraries(sys_unit_test PkgConfig::GTest ${CMAKE_THREAD_LIBS_INIT})

    add_test(NAME bear::sys_unit_test COMMAND $<TARGET_FILE:sys_unit_test>)
endif ()
