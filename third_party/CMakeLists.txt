# Some systems are using different library directory names for
# 64 bit and 32 bit libraries.
if (NOT LIBRARY_DIRECTORY_NAME)
    set(LIBRARY_DIRECTORY_NAME "lib64")
endif ()

message(STATUS "Looking for nlohman_json dependency")
find_package(nlohmann_json 3.7.3 QUIET)
if (nlohmann_json_FOUND)
    message(STATUS "Looking for nlohman_json dependency -- found")
    add_custom_target(nlohmann_json_dependency)
else ()
    message(STATUS "Looking for nlohman_json dependency -- not found")

    include(ExternalProject)
    ExternalProject_Add(nlohmann_json_dependency
            SOURCE_DIR
                "${CMAKE_CURRENT_SOURCE_DIR}/nlohmann_json"
            CMAKE_CACHE_ARGS
                -DJSON_Install:BOOL=ON
                -DJSON_BuildTests:BOOL=OFF
                -DCMAKE_INSTALL_PREFIX:PATH=${DEPENDENCIES_INSTALL_PREFIX}/nlohmann_json
            )
#    set(json_CMAKE_DIR "${DEPENDENCIES_INSTALL_PREFIX}/nlohmann_json/${LIBRARY_DIRECTORY_NAME}/cmake/nlohmann_json/")
endif ()


message(STATUS "Looking for fmt dependency")
find_package(fmt 6.2.0 QUIET)
if (fmt_FOUND)
    message(STATUS "Looking for fmt dependency -- found")
    add_custom_target(fmt_dependency)
else ()
    message(STATUS "Looking for fmt dependency -- not found")

    include(ExternalProject)
    ExternalProject_Add(fmt_dependency
            SOURCE_DIR
                "${CMAKE_CURRENT_SOURCE_DIR}/fmt"
            CMAKE_CACHE_ARGS
                -DFMT_INSTALL:BOOL=ON
                -DFMT_TEST:BOOL=OFF
                -DFMT_FUZZ:BOOL=OFF
                -DFMT_DOC:BOOL=OFF
                -DCMAKE_INSTALL_PREFIX:PATH=${DEPENDENCIES_INSTALL_PREFIX}/fmt
            )
#    set(fmt_CMAKE_DIR "${DEPENDENCIES_INSTALL_PREFIX}/fmt/${LIBRARY_DIRECTORY_NAME}/cmake/fmt/")
endif ()


message(STATUS "Looking for spdlog dependency")
find_package(spdlog 1.5.0 QUIET)
if (spdlog_FOUND)
    message(STATUS "Looking for spdlog dependency -- found")
    add_custom_target(spdlog_dependency)
else ()
    message(STATUS "Looking for spdlog dependency -- not found")

    include(ExternalProject)
    ExternalProject_Add(spdlog_dependency
            SOURCE_DIR
                "${CMAKE_CURRENT_SOURCE_DIR}/spdlog"
            DEPENDS
                fmt
            CMAKE_CACHE_ARGS
                -DSPDLOG_FMT_EXTERNAL:BOOL=ON
#                -Dfmt_DIR:PATH=${fmt_CMAKE_DIR}
                -DSPDLOG_INSTALL:BOOL=ON
                -DSPDLOG_NO_EXCEPTIONS:BOOL=ON
                -DSPDLOG_BUILD_TESTS:BOOL=OFF
                -DSPDLOG_BUILD_EXAMPLE:BOOL=OFF
                -DCMAKE_INSTALL_PREFIX:PATH=${DEPENDENCIES_INSTALL_PREFIX}/spdlog
            )
#    set(spdlog_CMAKE_DIR "${DEPENDENCIES_INSTALL_PREFIX}/spdlog/lib/cmake/spdlog/")
endif ()


message(STATUS "Looking for gRPC::grpc++ dependency")
find_package(PkgConfig REQUIRED)
pkg_check_modules(gRPC grpc++>=1.26) # make it bigger the upstream has a bug
if (gRPC_FOUND)
    message(STATUS "Looking for gRPC::grpc++ dependency -- found")

    message(STATUS "Looking for protoc")
    find_program(PROTOC protoc)
    if (PROTOC)
        message(STATUS "Looking for protoc -- found")
    else()
        message(FATAL_ERROR "Looking for protoc -- not found")
    endif()

    message(STATUS "Looking for grpc_cpp_plugin")
    find_program(GRPC_CPP_PLUGIN grpc_cpp_plugin)
    if (GRPC_CPP_PLUGIN)
        message(STATUS "Looking for grpc_cpp_plugin -- found")
    else()
        message(FATAL_ERROR "Looking for grpc_cpp_plugin -- not found")
    endif()

    add_custom_target(grpc_dependency)
else ()
    message(STATUS "Looking for gRPC::grpc++ dependency -- not found")

    include(ExternalProject)
    ExternalProject_Add(grpc_dependency
            SOURCE_DIR
                "${CMAKE_CURRENT_SOURCE_DIR}/grpc"
            CMAKE_CACHE_ARGS
                -DgRPC_INSTALL:BOOL=ON
                -DgRPC_BUILD_TESTS:BOOL=OFF
                -DgRPC_BUILD_CSHARP_EXT:BOOL=OFF
                -DCMAKE_INSTALL_PREFIX:PATH=${DEPENDENCIES_INSTALL_PREFIX}/grpc
            )
#    set(protobuf_CMAKE_DIR "${DEPENDENCIES_INSTALL_PREFIX}/grpc/${LIBRARY_DIRECTORY_NAME}/cmake/protobuf/")
#    set(grpc_CMAKE_DIR "${DEPENDENCIES_INSTALL_PREFIX}/grpc/lib/cmake/grpc/")
#    set(grpc_BIN_DIR "${DEPENDENCIES_INSTALL_PREFIX}/grpc/bin/")
endif ()


if (ENABLE_UNIT_TESTS)
    message(STATUS "Looking for GTest dependency")

    find_package(PkgConfig REQUIRED)
    pkg_check_modules(GTest IMPORTED_TARGET gtest>=1.10 gtest_main>=1.10 gmock>=1.10)

    if (GTest_FOUND)
        message(STATUS "Looking for GTest dependency -- found")
        add_custom_target(googletest_dependency)
    else ()
        message(STATUS "Looking for GTest dependency -- not found")

        include(ExternalProject)
        ExternalProject_Add(googletest_dependency
                SOURCE_DIR
                    "${CMAKE_CURRENT_SOURCE_DIR}/googletest"
                CMAKE_CACHE_ARGS
                    -DINSTALL_GTEST:BOOL=ON
                    -DCMAKE_INSTALL_PREFIX:PATH=${DEPENDENCIES_INSTALL_PREFIX}/googletest
                )
#        set(googletest_CMAKE_DIR "${DEPENDENCIES_INSTALL_PREFIX}/googletest/${LIBRARY_DIRECTORY_NAME}/cmake/GTest/")
    endif ()
else ()
    add_custom_target(googletest)
endif ()

