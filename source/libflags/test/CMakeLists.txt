add_executable(flags_unit_test
        FlagsTest.cc
        )

target_include_directories(flags_unit_test PRIVATE GTest::gtest)
target_compile_features(flags_unit_test PUBLIC cxx_std_17)
target_link_libraries(flags_unit_test result_a)
target_link_libraries(flags_unit_test flags_a)
target_link_libraries(flags_unit_test GTest::gtest GTest::gtest_main ${CMAKE_THREAD_LIBS_INIT})

add_test(NAME bear::flags_unit_test COMMAND flags_unit_test)
