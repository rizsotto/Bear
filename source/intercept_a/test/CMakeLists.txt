add_executable(libpear_unit_test
        ResultTest.cc
)

target_link_libraries(libpear_unit_test libgtest)
target_link_libraries(libpear_unit_test gtest_main_a)
target_link_libraries(libpear_unit_test intercept_a)

enable_testing()
add_test(NAME LibPearUnitTests COMMAND libpear_unit_test)
