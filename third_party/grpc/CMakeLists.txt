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
                v1.49.2
            GIT_SUBMODULES
            GIT_SHALLOW
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
                -DCMAKE_INSTALL_PREFIX:PATH=${DEPENDENCIES_INSTALL_PREFIX}/grpc_dependency
            CMAKE_CACHE_ARGS
                -DCMAKE_BUILD_TYPE:STRING=${CMAKE_BUILD_TYPE}
                -DCMAKE_TOOLCHAIN_FILE:PATH=${CMAKE_TOOLCHAIN_FILE}
                -DCMAKE_FIND_ROOT_PATH:PATH=${CMAKE_FIND_ROOT_PATH}
                -DCMAKE_C_COMPILER:STRING=${CMAKE_C_COMPILER}
                -DCMAKE_C_COMPILER_TARGET:STRING=${CMAKE_C_COMPILER_TARGET}
                -DCMAKE_C_FLAGS:STRING=${CMAKE_C_FLAGS}
                -DCMAKE_CXX_COMPILER:STRING=${CMAKE_CXX_COMPILER}
                -DCMAKE_CXX_COMPILER_TARGET:STRING=${CMAKE_CXX_COMPILER_TARGET}
                -DCMAKE_CXX_STANDARD:STRING=${CMAKE_CXX_STANDARD}
                -DCMAKE_CXX_STANDARD_REQUIRED:BOOL=${CMAKE_CXX_STANDARD_REQUIRED}
                -DCMAKE_CXX_EXTENSIONS:BOOL=${CMAKE_CXX_EXTENSIONS}
                -DCMAKE_CXX_FLAGS:STRING=${CMAKE_CXX_FLAGS}
                -DCMAKE_EXE_LINKER_FLAGS:STRING=${CMAKE_EXE_LINKER_FLAGS}
                -DCMAKE_SHARED_LINKER_FLAGS:STRING=${CMAKE_SHARED_LINKER_FLAGS}
                -DCMAKE_MODULE_LINKER_FLAGS:STRING=${CMAKE_MODULE_LINKER_FLAGS}
            )

    include(GNUInstallDirs)
    list(APPEND PREFIX_PATH "${DEPENDENCIES_INSTALL_PREFIX}/grpc_dependency")
    set(CMAKE_PREFIX_PATH ${PREFIX_PATH};${CMAKE_PREFIX_PATH}
            CACHE PATH "append gRPC libraries and binaries into the search path"
            FORCE)
endif ()
