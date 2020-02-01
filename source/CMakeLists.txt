enable_testing()
find_package(GTest REQUIRED)
find_package(Threads REQUIRED)

add_subdirectory(intercept)
add_subdirectory(intercept-wrapper)
add_subdirectory(intercept-library)
