cmake_minimum_required(VERSION 3.1...3.15)
if (${CMAKE_VERSION} VERSION_LESS 3.12)
    cmake_policy(VERSION ${CMAKE_MAJOR_VERSION}.${CMAKE_MINOR_VERSION})
else ()
    cmake_policy(VERSION 3.12)
endif ()

project(Bear-SuperBuild C CXX)

option(USE_SYSTEM_JSON "Use system installed JSON library" OFF)
option(USE_SYSTEM_GTEST "Use system installed GoogleTest library" OFF)
option(USE_SYSTEM_GRPC "Use system installed gRPC" OFF)

include(ExternalProject)

ExternalProject_Add(json
        PREFIX json
        SOURCE_DIR "${CMAKE_CURRENT_SOURCE_DIR}/../third_party/json"
        CMAKE_CACHE_ARGS
        -DJSON_Install:BOOL=ON
        -DJSON_BuildTests:BOOL=OFF
        -DCMAKE_INSTALL_PREFIX:PATH=${CMAKE_CURRENT_BINARY_DIR}/json
        )

ExternalProject_Add(googletest
        PREFIX json
        SOURCE_DIR "${CMAKE_CURRENT_SOURCE_DIR}/../third_party/googletest"
        CMAKE_CACHE_ARGS
        -DINSTALL_GTEST:BOOL=ON
        -DCMAKE_INSTALL_PREFIX:PATH=${CMAKE_CURRENT_BINARY_DIR}/googletest
        )

ExternalProject_Add(grpc
        PREFIX grpc
        SOURCE_DIR "${CMAKE_CURRENT_SOURCE_DIR}/../third_party/grpc"
        CMAKE_CACHE_ARGS
        -DgRPC_INSTALL:BOOL=ON
        -DgRPC_BUILD_TESTS:BOOL=OFF
        -DgRPC_BUILD_CSHARP_EXT:BOOL=OFF
        -DCMAKE_INSTALL_PREFIX:PATH=${CMAKE_CURRENT_BINARY_DIR}/grpc
        )

ExternalProject_Add(Bear
        PREFIX bear
        SOURCE_DIR "${CMAKE_CURRENT_SOURCE_DIR}/.."
        BINARY_DIR "${CMAKE_CURRENT_BINARY_DIR}/bear"
        INSTALL_COMMAND ""
        TEST_BEFORE_INSTALL 1
        CMAKE_CACHE_ARGS
        -Dnlohmann_json_DIR:PATH=${CMAKE_CURRENT_BINARY_DIR}/json/lib64/cmake/nlohmann_json/
        -DGTest_DIR:PATH=${CMAKE_CURRENT_BINARY_DIR}/googletest/lib64/cmake/GTest/
        -DProtobuf_DIR:PATH=${CMAKE_CURRENT_BINARY_DIR}/grpc/lib64/cmake/protobuf/
        -DgRPC_DIR:PATH=${CMAKE_CURRENT_BINARY_DIR}/grpc/lib/cmake/grpc/
        -DgRPC_BINDIR:PATH=${CMAKE_CURRENT_BINARY_DIR}/grpc/bin/
        DEPENDS json googletest grpc
        )
