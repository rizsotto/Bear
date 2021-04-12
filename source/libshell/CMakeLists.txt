add_library(shell_a OBJECT)
target_include_directories(shell_a PUBLIC include/)
target_sources(shell_a
        PRIVATE
            source/Command.cc
        INTERFACE
            $<TARGET_OBJECTS:shell_a>
        )
target_link_libraries(shell_a PUBLIC result_a)

if (ENABLE_UNIT_TESTS)
    add_executable(shell_unit_test
            test/CommandTest.cc
            )

    target_link_libraries(shell_unit_test shell_a)
    target_link_libraries(shell_unit_test PkgConfig::GTest ${CMAKE_THREAD_LIBS_INIT})

    add_test(NAME bear::shell_unit_test COMMAND $<TARGET_FILE:shell_unit_test>)
endif ()
