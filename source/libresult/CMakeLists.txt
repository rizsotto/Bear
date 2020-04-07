add_library(result_a INTERFACE
        )

target_include_directories(result_a INTERFACE include/)
target_compile_features(result_a INTERFACE cxx_std_17)
target_compile_options(result_a INTERFACE -fno-exceptions -fno-rtti)

add_executable(result_unit_test
        test/ResultTest.cc
        )

target_link_libraries(result_unit_test result_a)
target_link_libraries(result_unit_test GTest::gtest GTest::gtest_main ${CMAKE_THREAD_LIBS_INIT})
target_link_libraries(result_unit_test GTest::gmock)

add_test(NAME bear::result_unit_test COMMAND result_unit_test)
