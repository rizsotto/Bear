file(GLOB SRCS *.cc)
add_executable(pear_test ${SRCS})
target_link_libraries(pear_test libgtest)

enable_testing()
add_test(NAME PearUnitTests COMMAND pear_test)
