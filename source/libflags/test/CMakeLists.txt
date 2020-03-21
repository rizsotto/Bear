add_executable(libflags_unit_test
        FlagsTest.cc
        )

target_include_directories(libflags_unit_test PRIVATE ${LIBRESULT_INCLUDE_DIR})
target_include_directories(libflags_unit_test PRIVATE GTest::gtest)
target_compile_features(libflags_unit_test PUBLIC cxx_std_17)
target_link_libraries(libflags_unit_test flags_a)
target_link_libraries(libflags_unit_test GTest::gtest GTest::gtest_main ${CMAKE_THREAD_LIBS_INIT})

add_test(NAME bear::libflags_unit_test COMMAND libflags_unit_test)
