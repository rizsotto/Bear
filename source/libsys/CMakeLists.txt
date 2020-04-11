add_library(sys_a OBJECT
        source/Context.cc
        source/Environment.cc
        )

target_include_directories(sys_a PUBLIC include/)
target_link_libraries(sys_a PUBLIC result_a)
target_link_libraries(sys_a PRIVATE fmt::fmt)

add_executable(sys_unit_test
        test/EnvironmentTest.cc
        test/ErrorsTest.cc
        )

target_include_directories(sys_unit_test PRIVATE source/)
target_link_libraries(sys_unit_test sys_a)
target_link_libraries(sys_unit_test result_a)
target_link_libraries(sys_unit_test fmt::fmt)
target_link_libraries(sys_unit_test GTest::gtest GTest::gtest_main ${CMAKE_THREAD_LIBS_INIT})

add_test(NAME bear::sys_unit_test COMMAND sys_unit_test)
