add_executable(libintercept_unit_test
        EnvironmentTest.cc
        ResultTest.cc
        SessionTest.cc
)

target_link_libraries(libintercept_unit_test libgtest)
target_link_libraries(libintercept_unit_test gtest_main)
target_link_libraries(libintercept_unit_test intercept_a)
target_link_libraries(libintercept_unit_test stdc++fs)

enable_testing()
add_test(NAME LibInterceptUnitTests COMMAND libintercept_unit_test)
