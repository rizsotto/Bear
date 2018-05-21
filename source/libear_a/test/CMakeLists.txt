add_executable(libear_unit_test
        ArrayTest.cc
        EnvironmentTest.cc
        ExecutorTest.cc
        StorageTest.cc
        StringTest.cc
)

target_link_libraries(libear_unit_test libgtest)
target_link_libraries(libear_unit_test gtest_main_a)
target_link_libraries(libear_unit_test ear_a)

enable_testing()
add_test(NAME LibEarUnitTests COMMAND libear_unit_test)
