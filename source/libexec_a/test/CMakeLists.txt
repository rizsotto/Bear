add_executable(libexec_unit_test
        ArrayTest.cc
        EnvironmentTest.cc
        ExecutorTest.cc
        StorageTest.cc
        StringTest.cc
)

target_link_libraries(libexec_unit_test libgtest)
target_link_libraries(libexec_unit_test gtest_main)
target_link_libraries(libexec_unit_test exec_a)

enable_testing()
add_test(NAME LibExecUnitTests COMMAND libexec_unit_test)
