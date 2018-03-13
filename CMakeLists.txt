cmake_minimum_required(VERSION 2.8)

project(bear C CXX)
set(BEAR_VERSION "3.0.0")

# We need thread support
find_package(Threads REQUIRED)

# Enable ExternalProject CMake module
include(ExternalProject)

# Download and install GoogleTest
ExternalProject_Add(
        gtest
        URL https://github.com/google/googletest/archive/release-1.8.0.zip
        PREFIX ${CMAKE_CURRENT_BINARY_DIR}/gtest
        # Disable install step
        INSTALL_COMMAND ""
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

# I couldn't make it work with INTERFACE_INCLUDE_DIRECTORIES
include_directories("${source_dir}/googletest/include"
        "${source_dir}/googlemock/include")


if (NOT CMAKE_BUILD_TYPE)
    set(CMAKE_BUILD_TYPE "Release")
endif()

set(EAR_LIB_FILE ${CMAKE_SHARED_LIBRARY_PREFIX}ear${CMAKE_SHARED_LIBRARY_SUFFIX})
set(EAR_LIB_PATH "${CMAKE_INSTALL_LIBDIR}/bear")
set(DEFAULT_PRELOAD_FILE ${CMAKE_INSTALL_PREFIX}/${EAR_LIB_PATH}/${EAR_LIB_FILE} CACHE STRING "Default path to libear.")

enable_testing()
set(CMAKE_CTEST_COMMAND ctest -V)
add_custom_target(check COMMAND ${CMAKE_CTEST_COMMAND})

add_subdirectory(bear)
add_subdirectory(test)
add_subdirectory(man)
add_subdirectory(libear_s)
add_subdirectory(pear)


include(GNUInstallDirs)
install(FILES COPYING README.md ChangeLog.md
        DESTINATION ${CMAKE_INSTALL_DOCDIR})

set(CPACK_PACKAGE_NAME "bear")
set(CPACK_PACKAGE_CONTACT "László Nagy")
set(CPACK_PACKAGE_VENDOR ${CPACK_PACKAGE_CONTACT})
set(CPACK_PACKAGE_VERSION ${BEAR_VERSION})
set(CPACK_PACKAGE_DESCRIPTION_SUMMARY "BuildEAR")
set(CPACK_RESOURCE_FILE_LICENSE "${CMAKE_CURRENT_SOURCE_DIR}/COPYING")
set(CPACK_RESOURCE_FILE_README "${CMAKE_CURRENT_SOURCE_DIR}/README.md")
set(CPACK_PACKAGING_INSTALL_PREFIX ${CMAKE_INSTALL_PREFIX})
set(CPACK_RPM_PACKAGE_RELEASE "1%{?dist}")
set(CPACK_RPM_PACKAGE_LICENSE "GPLv3")
set(CPACK_RPM_PACKAGE_GROUP "Development/Tools")
set(CPACK_RPM_PACKAGE_URL "http://github.com/rizsotto/Bear")
set(CPACK_RPM_PACKAGE_DESCRIPTION "Bear is a tool to generate compilation database for clang tooling.")
set(CPACK_RPM_EXCLUDE_FROM_AUTO_FILELIST_ADDITION
    "${CMAKE_INSTALL_MANDIR}"
    "${CMAKE_INSTALL_MANDIR}/man1")
set(CPACK_DEBIAN_PACKAGE_SHLIBDEPS ON)
include(CPack)
