file(GLOB SRCS *.cc)
add_executable(ear_test ${SRCS})
target_link_libraries(ear_test libgtest)

enable_testing()
add_test(NAME EarUnitTests COMMAND ear_test)
