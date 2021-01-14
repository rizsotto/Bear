message(STATUS "Looking for gRPC::grpc++ dependency")
find_package(PkgConfig REQUIRED)
pkg_check_modules(gRPC protobuf>=3.11 grpc++>=1.26)
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
            GIT_REPOSITORY
                https://github.com/grpc/grpc
            GIT_TAG
                v1.34.1
            GIT_SUBMODULES
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
                -DCMAKE_INSTALL_PREFIX:PATH=${DEPENDENCIES_INSTALL_PREFIX}/grpc_dependency
            )

    include(GNUInstallDirs)
    list(APPEND PREFIX_PATH "${DEPENDENCIES_INSTALL_PREFIX}/grpc_dependency")
    set(CMAKE_PREFIX_PATH ${CMAKE_PREFIX_PATH};${PREFIX_PATH}
            CACHE PATH "append gRPC libraries and binaries into the search path"
            FORCE)
endif ()
