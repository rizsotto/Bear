add_library(flags_a OBJECT
        source/Flags.cc
        )

target_include_directories(flags_a PUBLIC include/)
target_link_libraries(flags_a PUBLIC result_a)
target_link_libraries(flags_a PRIVATE fmt::fmt)

if (ENABLE_UNIT_TESTS)
    add_executable(flags_unit_test
            test/FlagsTest.cc
            )

    target_link_libraries(flags_unit_test result_a)
    target_link_libraries(flags_unit_test flags_a)
    target_link_libraries(flags_unit_test fmt::fmt)
    target_link_libraries(flags_unit_test GTest::gtest GTest::gtest_main ${CMAKE_THREAD_LIBS_INIT})

    add_test(NAME bear::flags_unit_test COMMAND $<TARGET_FILE:flags_unit_test>)
endif ()