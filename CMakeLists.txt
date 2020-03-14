cmake_minimum_required(VERSION 3.1...3.16)

project(Bear-SuperBuild C CXX)

option(USE_SYSTEM_JSON "Use system installed JSON library" OFF)
option(USE_SYSTEM_GTEST "Use system installed GoogleTest library" OFF)
option(USE_SYSTEM_GRPC "Use system installed gRPC" OFF)

include(ExternalProject)

if (NOT USE_SYSTEM_JSON)
    ExternalProject_Add(json
            PREFIX json
            SOURCE_DIR "${CMAKE_CURRENT_SOURCE_DIR}/third_party/json"
            CMAKE_CACHE_ARGS
            -DJSON_Install:BOOL=ON
            -DJSON_BuildTests:BOOL=OFF
            -DCMAKE_INSTALL_PREFIX:PATH=${CMAKE_CURRENT_BINARY_DIR}/json
            )
    set(json_CMAKE_DIR "${CMAKE_CURRENT_BINARY_DIR}/json/lib64/cmake/nlohmann_json/")
endif ()

if (NOT USE_SYSTEM_GTEST)
    ExternalProject_Add(googletest
            PREFIX googletest
            SOURCE_DIR "${CMAKE_CURRENT_SOURCE_DIR}/third_party/googletest"
            CMAKE_CACHE_ARGS
            -DINSTALL_GTEST:BOOL=ON
            -DCMAKE_INSTALL_PREFIX:PATH=${CMAKE_CURRENT_BINARY_DIR}/googletest
            )
    set(googletest_CMAKE_DIR "${CMAKE_CURRENT_BINARY_DIR}/googletest/lib64/cmake/GTest/")
endif ()

if (NOT USE_SYSTEM_GRPC)
    ExternalProject_Add(grpc
            PREFIX grpc
            SOURCE_DIR "${CMAKE_CURRENT_SOURCE_DIR}/third_party/grpc"
            CMAKE_CACHE_ARGS
            -DgRPC_INSTALL:BOOL=ON
            -DgRPC_BUILD_TESTS:BOOL=OFF
            -DgRPC_BUILD_CSHARP_EXT:BOOL=OFF
            -DCMAKE_INSTALL_PREFIX:PATH=${CMAKE_CURRENT_BINARY_DIR}/grpc
            )
    set(protobuf_CMAKE_DIR "${CMAKE_CURRENT_BINARY_DIR}/grpc/lib64/cmake/protobuf/")
    set(grpc_CMAKE_DIR "${CMAKE_CURRENT_BINARY_DIR}/grpc/lib/cmake/grpc/")
    set(grpc_BIN_DIR "${CMAKE_CURRENT_BINARY_DIR}/grpc/bin/")
endif ()

ExternalProject_Add(Bear
        PREFIX bear
        SOURCE_DIR "${CMAKE_CURRENT_SOURCE_DIR}/source"
        BINARY_DIR "${CMAKE_CURRENT_BINARY_DIR}/bear"
        INSTALL_COMMAND ""
        TEST_BEFORE_INSTALL 1
        CMAKE_CACHE_ARGS
        -Dnlohmann_json_DIR:PATH=${json_CMAKE_DIR}
        -DGTest_DIR:PATH=${googletest_CMAKE_DIR}
        -DProtobuf_DIR:PATH=${protobuf_CMAKE_DIR}
        -DgRPC_DIR:PATH=${grpc_CMAKE_DIR}
        -DgRPC_BINDIR:PATH=${grpc_BIN_DIR}
        DEPENDS json googletest grpc
        )
