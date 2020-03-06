add_executable(libresult_unit_test
        ../include/Result.h
        ResultTest.cc
        )

target_include_directories(libresult_unit_test PRIVATE ${CMAKE_CURRENT_SOURCE_DIR}/../include)
target_include_directories(libresult_unit_test PRIVATE GTest::gtest)
target_include_directories(libresult_unit_test PRIVATE GMock::gmock)
target_compile_features(libresult_unit_test PUBLIC cxx_std_14)
target_link_libraries(libresult_unit_test GTest::gtest GTest::gtest_main ${CMAKE_THREAD_LIBS_INIT})
target_link_libraries(libresult_unit_test GTest::gmock)

add_test(NAME bear::libresult_unit_test COMMAND libresult_unit_test)
