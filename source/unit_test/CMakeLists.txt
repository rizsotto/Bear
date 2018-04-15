# We need thread support
find_package(Threads REQUIRED)

# Enable ExternalProject CMake module
include(ExternalProject)

# Download and install GoogleTest
ExternalProject_Add(
        gtest
        URL https://github.com/google/googletest/archive/release-1.8.0.zip
        URL_HASH MD5=adfafc8512ab65fd3cf7955ef0100ff5
        PREFIX ${CMAKE_CURRENT_BINARY_DIR}/gtest
        INSTALL_COMMAND ""  # Disable install step
)

# Get GTest source and binary directories from CMake project
ExternalProject_Get_Property(gtest source_dir binary_dir)

# Create a libgtest target to be used as a dependency by test programs
add_library(libgtest IMPORTED STATIC GLOBAL)
add_dependencies(libgtest gtest)

# Set libgtest properties
set_target_properties(libgtest PROPERTIES
        "IMPORTED_LOCATION" "${binary_dir}/googlemock/gtest/libgtest.a"
        "IMPORTED_LINK_INTERFACE_LIBRARIES" "${CMAKE_THREAD_LIBS_INIT}"
        )

# Create a libgmock target to be used as a dependency by test programs
add_library(libgmock IMPORTED STATIC GLOBAL)
add_dependencies(libgmock gtest)

# Set libgmock properties
set_target_properties(libgmock PROPERTIES
        "IMPORTED_LOCATION" "${binary_dir}/googlemock/libgmock.a"
        "IMPORTED_LINK_INTERFACE_LIBRARIES" "${CMAKE_THREAD_LIBS_INIT}"
        )

set(GOOGLE_TEST_INCLUDE_DIR "${source_dir}/googletest/include")
set(GOOGLE_MOCK_INCLUDE_DIR "${source_dir}/googlemock/include")


# Use the gtest framework for unit testing
file(GLOB SRCS *.cc)
add_executable(unit_test ${SRCS})

include_directories(${GOOGLE_TEST_INCLUDE_DIR})
target_link_libraries(unit_test libgtest)
target_link_libraries(unit_test ear_a)

enable_testing()
add_test(NAME UnitTests COMMAND unit_test)
