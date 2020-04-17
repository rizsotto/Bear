cmake_minimum_required(VERSION 3.2)

project(Bear-SuperBuild C CXX)

option(USE_SYSTEM_FRUIT "Use system installed Fruit library" OFF)
option(USE_SYSTEM_JSON "Use system installed JSON library" OFF)
option(USE_SYSTEM_GTEST "Use system installed GoogleTest library" OFF)
option(USE_SYSTEM_FMT "Use system installed fmt library" OFF)
option(USE_SYSTEM_SPDLOG "Use system installed spdlog library" OFF)
option(USE_SYSTEM_GRPC "Use system installed gRPC" OFF)

# Some systems are using different library directory names for
# 64 bit and 32 bit libraries.
if (NOT LIBRARY_DIRECTORY_NAME)
    set(LIBRARY_DIRECTORY_NAME "lib64")
endif ()

include(ExternalProject)

if (NOT USE_SYSTEM_FRUIT)
    ExternalProject_Add(fruit
            PREFIX fruit
            SOURCE_DIR "${CMAKE_CURRENT_SOURCE_DIR}/third_party/fruit"
            CMAKE_CACHE_ARGS
            -DFRUIT_USES_BOOST:BOOL=False
            -DCMAKE_CXX_FLAGS:STRING=-fno-rtti
            -DBUILD_SHARED_LIBS:BOOL=OFF
            -DCMAKE_BUILD_TYPE:STRING=Release
            -DCMAKE_INSTALL_PREFIX:PATH=${CMAKE_CURRENT_BINARY_DIR}/fruit
            )
    set(fruit_CMAKE_DIR "${CMAKE_CURRENT_BINARY_DIR}/fruit/")
endif ()

if (NOT USE_SYSTEM_JSON)
    ExternalProject_Add(json
            PREFIX json
            SOURCE_DIR "${CMAKE_CURRENT_SOURCE_DIR}/third_party/json"
            CMAKE_CACHE_ARGS
            -DJSON_Install:BOOL=ON
            -DJSON_BuildTests:BOOL=OFF
            -DCMAKE_INSTALL_PREFIX:PATH=${CMAKE_CURRENT_BINARY_DIR}/json
            )
    set(json_CMAKE_DIR "${CMAKE_CURRENT_BINARY_DIR}/json/${LIBRARY_DIRECTORY_NAME}/cmake/nlohmann_json/")
endif ()

if (NOT USE_SYSTEM_GTEST)
    ExternalProject_Add(googletest
            PREFIX googletest
            SOURCE_DIR "${CMAKE_CURRENT_SOURCE_DIR}/third_party/googletest"
            CMAKE_CACHE_ARGS
            -DINSTALL_GTEST:BOOL=ON
            -DCMAKE_INSTALL_PREFIX:PATH=${CMAKE_CURRENT_BINARY_DIR}/googletest
            )
    set(googletest_CMAKE_DIR "${CMAKE_CURRENT_BINARY_DIR}/googletest/${LIBRARY_DIRECTORY_NAME}/cmake/GTest/")
endif ()

if (NOT USE_SYSTEM_FMT)
    ExternalProject_Add(fmt
            PREFIX grpc
            SOURCE_DIR "${CMAKE_CURRENT_SOURCE_DIR}/third_party/fmt"
            CMAKE_CACHE_ARGS
            -DFMT_INSTALL:BOOL=ON
            -DFMT_TEST:BOOL=OFF
            -DFMT_FUZZ:BOOL=OFF
            -DFMT_DOC:BOOL=OFF
            -DCMAKE_INSTALL_PREFIX:PATH=${CMAKE_CURRENT_BINARY_DIR}/fmt
            )
    set(fmt_CMAKE_DIR "${CMAKE_CURRENT_BINARY_DIR}/fmt/${LIBRARY_DIRECTORY_NAME}/cmake/fmt/")
endif ()

if (NOT USE_SYSTEM_SPDLOG)
    ExternalProject_Add(spdlog
            PREFIX grpc
            SOURCE_DIR "${CMAKE_CURRENT_SOURCE_DIR}/third_party/spdlog"
            CMAKE_CACHE_ARGS
            -DSPDLOG_FMT_EXTERNAL:BOOL=ON
            -Dfmt_DIR:PATH=${fmt_CMAKE_DIR}
            -DSPDLOG_INSTALL:BOOL=ON
            -DSPDLOG_NO_EXCEPTIONS:BOOL=ON
            -DSPDLOG_BUILD_TESTS:BOOL=OFF
            -DSPDLOG_BUILD_EXAMPLE:BOOL=OFF
            -DCMAKE_INSTALL_PREFIX:PATH=${CMAKE_CURRENT_BINARY_DIR}/spdlog
            DEPENDS fmt
            )
    set(spdlog_CMAKE_DIR "${CMAKE_CURRENT_BINARY_DIR}/spdlog/${LIBRARY_DIRECTORY_NAME}/cmake/spdlog/")
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
    set(protobuf_CMAKE_DIR "${CMAKE_CURRENT_BINARY_DIR}/grpc/${LIBRARY_DIRECTORY_NAME}/cmake/protobuf/")
    set(grpc_CMAKE_DIR "${CMAKE_CURRENT_BINARY_DIR}/grpc/lib/cmake/grpc/")
    set(grpc_BIN_DIR "${CMAKE_CURRENT_BINARY_DIR}/grpc/bin/")
endif ()

ExternalProject_Add(Bear
        PREFIX bear
        SOURCE_DIR "${CMAKE_CURRENT_SOURCE_DIR}/source"
        TEST_BEFORE_INSTALL 1
        CMAKE_CACHE_ARGS
        -DFRUIT_INSTALLED_DIR:PATH=${fruit_CMAKE_DIR}
        -Dnlohmann_json_DIR:PATH=${json_CMAKE_DIR}
        -DGTest_DIR:PATH=${googletest_CMAKE_DIR}
        -DProtobuf_DIR:PATH=${protobuf_CMAKE_DIR}
        -DgRPC_DIR:PATH=${grpc_CMAKE_DIR}
        -DgRPC_BINDIR:PATH=${grpc_BIN_DIR}
        -Dfmt_DIR:PATH=${fmt_CMAKE_DIR}
        -Dspdlog_DIR:PATH=${spdlog_CMAKE_DIR}
        -DCMAKE_INSTALL_PREFIX:PATH=${CMAKE_CURRENT_BINARY_DIR}/bear
        DEPENDS fruit json googletest fmt spdlog grpc
        )
set(BEAR_BIN_DIR "${CMAKE_CURRENT_BINARY_DIR}/bear/bin")
set(BEAR_LIB_DIR "${CMAKE_CURRENT_BINARY_DIR}/bear/${LIBRARY_DIRECTORY_NAME}")
set(BEAR_LIBEXEC_DIR "${CMAKE_CURRENT_BINARY_DIR}/bear/libexec")

ExternalProject_Add(BearTest
        PREFIX test
        INSTALL_COMMAND ""
        SOURCE_DIR "${CMAKE_CURRENT_SOURCE_DIR}/test"
        TEST_BEFORE_INSTALL 1
        CMAKE_CACHE_ARGS
        -D_BEAR_BIN_DIR:PATH=${BEAR_BIN_DIR}
        -D_BEAR_LIB_DIR:PATH=${BEAR_LIB_DIR}
        -D_BEAR_LIBEXEC_DIR:PATH=${BEAR_LIBEXEC_DIR}
        DEPENDS Bear
        )
