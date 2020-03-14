cmake_minimum_required(VERSION 3.1...3.16)
if (${CMAKE_VERSION} VERSION_LESS 3.12)
    cmake_policy(VERSION ${CMAKE_MAJOR_VERSION}.${CMAKE_MINOR_VERSION})
else ()
    cmake_policy(VERSION 3.12)
endif ()

set(default_build_type "Release")
if (NOT CMAKE_BUILD_TYPE AND NOT CMAKE_CONFIGURATION_TYPES)
    message(STATUS "Setting build type to '${default_build_type}' as none was specified.")
    set(CMAKE_BUILD_TYPE "${default_build_type}" CACHE
            STRING "Choose the type of build." FORCE)
    # Set the possible values of build type for cmake-gui
    set_property(CACHE CMAKE_BUILD_TYPE PROPERTY STRINGS
            "Debug" "Release" "MinSizeRel" "RelWithDebInfo")
endif ()


project(bear
        VERSION 3.0.0
        DESCRIPTION "Bear is a tool to generate compilation database for clang tooling."
        LANGUAGES C CXX)

enable_testing()

find_package(Threads REQUIRED)
find_package(nlohmann_json REQUIRED)
find_package(GTest REQUIRED NO_CMAKE_PATH)
find_package(Protobuf REQUIRED NO_CMAKE_PATH)
find_package(gRPC REQUIRED NO_CMAKE_PATH)

find_program(_PROTOBUF_PROTOC protoc HINTS ${gRPC_BINDIR})
message(STATUS "Looking for protoc ... ${_PROTOBUF_PROTOC}")
find_program(_GRPC_CPP_PLUGIN_EXECUTABLE grpc_cpp_plugin HINTS ${gRPC_BINDIR})
message(STATUS "Looking for grpc_cpp_plugin ... ${_GRPC_CPP_PLUGIN_EXECUTABLE}")

add_subdirectory(bear)
add_subdirectory(intercept)
add_subdirectory(intercept-wrapper)
add_subdirectory(intercept-library)

include(GNUInstallDirs)
install(FILES ../COPYING ../README.md ../ChangeLog.md
        DESTINATION ${CMAKE_INSTALL_DOCDIR})

set(CPACK_PACKAGE_NAME "bear")
set(CPACK_PACKAGE_CONTACT "László Nagy")
set(CPACK_PACKAGE_VENDOR ${CPACK_PACKAGE_CONTACT})
set(CPACK_PACKAGE_VERSION ${BEAR_VERSION})
set(CPACK_PACKAGE_DESCRIPTION_SUMMARY "BuildEAR")
set(CPACK_RESOURCE_FILE_LICENSE "${CMAKE_CURRENT_SOURCE_DIR}/../COPYING")
set(CPACK_RESOURCE_FILE_README "${CMAKE_CURRENT_SOURCE_DIR}/../README.md")
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
