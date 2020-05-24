message(STATUS "Looking for gRPC::grpc++ dependency")
find_package(PkgConfig REQUIRED)
pkg_check_modules(gRPC grpc++>=1.26)
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
            URL
                https://github.com/grpc/grpc/archive/v1.29.1.tar.gz
            URL_HASH
                MD5=5bd3155249be19372e236f9a3b4bfdbd
            DOWNLOAD_NO_PROGRESS
                1
            UPDATE_COMMAND
                ""
            LOG_CONFIGURE
                1
            LOG_BUILD
                1
            LOG_INSTALL
                1
            CMAKE_ARGS
                -DgRPC_INSTALL:BOOL=ON
                -DgRPC_BUILD_TESTS:BOOL=OFF
                -DgRPC_BUILD_CSHARP_EXT:BOOL=OFF
                -DCMAKE_INSTALL_PREFIX:PATH=${DEPENDENCIES_INSTALL_PREFIX}/grpc
            )
    #    set(protobuf_CMAKE_DIR "${DEPENDENCIES_INSTALL_PREFIX}/grpc/${LIBRARY_DIRECTORY_NAME}/cmake/protobuf/")
    #    set(grpc_CMAKE_DIR "${DEPENDENCIES_INSTALL_PREFIX}/grpc/lib/cmake/grpc/")
    #    set(grpc_BIN_DIR "${DEPENDENCIES_INSTALL_PREFIX}/grpc/bin/")
endif ()
