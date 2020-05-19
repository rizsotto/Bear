cmake_minimum_required(VERSION 3.2)
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

project(Bear-SuperBuild
        VERSION 3.0.0
        LANGUAGES C CXX
        )

option(ENABLE_UNIT_TESTS "Build and run unit test for this project" ON)
option(ENABLE_FUNC_TESTS "Build and run functional test for this project" ON)

option(USE_SYSTEM_JSON "Use system installed JSON library" OFF)
option(USE_SYSTEM_GTEST "Use system installed GoogleTest library" OFF)
option(USE_SYSTEM_FMT "Use system installed fmt library" OFF)
option(USE_SYSTEM_SPDLOG "Use system installed spdlog library" OFF)
option(USE_SYSTEM_GRPC "Use system installed gRPC" OFF)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_EXTENSIONS OFF)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

set_property(DIRECTORY PROPERTY EP_BASE ${CMAKE_BINARY_DIR}/subprojects)
set(STAGED_INSTALL_PREFIX ${CMAKE_BINARY_DIR}/stage)
set(DEPENDENCIES_INSTALL_PREFIX ${CMAKE_BINARY_DIR}/subprojects/Install)

include(ExternalProject)

# Some systems are using different library directory names for
# 64 bit and 32 bit libraries.
if (NOT LIBRARY_DIRECTORY_NAME)
    set(LIBRARY_DIRECTORY_NAME "lib64")
endif ()


if (ENABLE_UNIT_TESTS)
    if (NOT USE_SYSTEM_GTEST)
        ExternalProject_Add(googletest
                SOURCE_DIR
                    "${CMAKE_CURRENT_SOURCE_DIR}/third_party/googletest"
                CMAKE_CACHE_ARGS
                    -DINSTALL_GTEST:BOOL=ON
                    -DCMAKE_INSTALL_PREFIX:PATH=${DEPENDENCIES_INSTALL_PREFIX}/googletest
                )
	    set(googletest_CMAKE_DIR "${DEPENDENCIES_INSTALL_PREFIX}/googletest/${LIBRARY_DIRECTORY_NAME}/cmake/GTest/")
    endif ()
else ()
    add_custom_target(googletest)
endif ()

if (NOT USE_SYSTEM_JSON)
    ExternalProject_Add(json
            SOURCE_DIR
                "${CMAKE_CURRENT_SOURCE_DIR}/third_party/json"
            CMAKE_CACHE_ARGS
                -DJSON_Install:BOOL=ON
                -DJSON_BuildTests:BOOL=OFF
                -DCMAKE_INSTALL_PREFIX:PATH=${DEPENDENCIES_INSTALL_PREFIX}/json
            )
    set(json_CMAKE_DIR "${DEPENDENCIES_INSTALL_PREFIX}/json/${LIBRARY_DIRECTORY_NAME}/cmake/nlohmann_json/")
endif ()

if (NOT USE_SYSTEM_FMT)
    ExternalProject_Add(fmt
            SOURCE_DIR
                "${CMAKE_CURRENT_SOURCE_DIR}/third_party/fmt"
            CMAKE_CACHE_ARGS
                -DFMT_INSTALL:BOOL=ON
                -DFMT_TEST:BOOL=OFF
                -DFMT_FUZZ:BOOL=OFF
                -DFMT_DOC:BOOL=OFF
                -DCMAKE_INSTALL_PREFIX:PATH=${DEPENDENCIES_INSTALL_PREFIX}/fmt
            )
    set(fmt_CMAKE_DIR "${DEPENDENCIES_INSTALL_PREFIX}/fmt/${LIBRARY_DIRECTORY_NAME}/cmake/fmt/")
endif ()

if (NOT USE_SYSTEM_SPDLOG)
    ExternalProject_Add(spdlog
            SOURCE_DIR
                "${CMAKE_CURRENT_SOURCE_DIR}/third_party/spdlog"
            DEPENDS
                fmt
            CMAKE_CACHE_ARGS
                -DSPDLOG_FMT_EXTERNAL:BOOL=ON
                -Dfmt_DIR:PATH=${fmt_CMAKE_DIR}
                -DSPDLOG_INSTALL:BOOL=ON
                -DSPDLOG_NO_EXCEPTIONS:BOOL=ON
                -DSPDLOG_BUILD_TESTS:BOOL=OFF
                -DSPDLOG_BUILD_EXAMPLE:BOOL=OFF
                -DCMAKE_INSTALL_PREFIX:PATH=${DEPENDENCIES_INSTALL_PREFIX}/spdlog
            )
    set(spdlog_CMAKE_DIR "${DEPENDENCIES_INSTALL_PREFIX}/spdlog/lib/cmake/spdlog/")
endif ()

if (NOT USE_SYSTEM_GRPC)
    ExternalProject_Add(grpc
            SOURCE_DIR
                "${CMAKE_CURRENT_SOURCE_DIR}/third_party/grpc"
            CMAKE_CACHE_ARGS
                -DgRPC_INSTALL:BOOL=ON
                -DgRPC_BUILD_TESTS:BOOL=OFF
                -DgRPC_BUILD_CSHARP_EXT:BOOL=OFF
		        -DCMAKE_INSTALL_PREFIX:PATH=${DEPENDENCIES_INSTALL_PREFIX}/grpc
            )
    set(protobuf_CMAKE_DIR "${DEPENDENCIES_INSTALL_PREFIX}/grpc/${LIBRARY_DIRECTORY_NAME}/cmake/protobuf/")
    set(grpc_CMAKE_DIR "${DEPENDENCIES_INSTALL_PREFIX}/grpc/lib/cmake/grpc/")
    set(grpc_BIN_DIR "${DEPENDENCIES_INSTALL_PREFIX}/grpc/bin/")
endif ()

ExternalProject_Add(Bear
        SOURCE_DIR
            "${CMAKE_CURRENT_SOURCE_DIR}/source"
        DEPENDS
            json googletest fmt spdlog grpc
        CMAKE_ARGS
            -DCMAKE_CXX_COMPILER=${CMAKE_CXX_COMPILER}
            -DCMAKE_CXX_STANDARD=${CMAKE_CXX_STANDARD}
            -DCMAKE_CXX_EXTENSIONS=${CMAKE_CXX_EXTENSIONS}
            -DCMAKE_CXX_STANDARD_REQUIRED=${CMAKE_CXX_STANDARD_REQUIRED}
            -DCMAKE_INSTALL_PREFIX:PATH=${STAGED_INSTALL_PREFIX}
        CMAKE_CACHE_ARGS
            -DENABLE_UNIT_TESTS:BOOL=${ENABLE_UNIT_TESTS}
            -DCMAKE_CXX_FLAGS:STRING=${CMAKE_CXX_FLAGS}
            -DCMAKE_PREFIX_PATH:PATH=${CMAKE_PREFIX_PATH}
#        CMAKE_CACHE_ARGS
            -Dnlohmann_json_DIR:PATH=${json_CMAKE_DIR}
            -DGTest_DIR:PATH=${googletest_CMAKE_DIR}
            -DProtobuf_DIR:PATH=${protobuf_CMAKE_DIR}
            -DgRPC_DIR:PATH=${grpc_CMAKE_DIR}
            -DgRPC_BINDIR:PATH=${grpc_BIN_DIR}
            -Dfmt_DIR:PATH=${fmt_CMAKE_DIR}
            -Dspdlog_DIR:PATH=${spdlog_CMAKE_DIR}
        BUILD_ALWAYS
            1
        TEST_BEFORE_INSTALL
            1
        TEST_COMMAND
            ctest
#            ctest -T memcheck
        )

if (ENABLE_FUNC_TESTS)
    set(BEAR_BIN_DIR "${STAGED_INSTALL_PREFIX}/bin")
    set(BEAR_LIB_DIR "${STAGED_INSTALL_PREFIX}/${LIBRARY_DIRECTORY_NAME}")
    set(BEAR_LIBEXEC_DIR "${STAGED_INSTALL_PREFIX}/libexec")

    ExternalProject_Add(BearTest
            SOURCE_DIR
                "${CMAKE_CURRENT_SOURCE_DIR}/test"
            DEPENDS
                Bear
            CMAKE_CACHE_ARGS
                -D_BEAR_BIN_DIR:PATH=${BEAR_BIN_DIR}
                -D_BEAR_LIB_DIR:PATH=${BEAR_LIB_DIR}
                -D_BEAR_LIBEXEC_DIR:PATH=${BEAR_LIBEXEC_DIR}
            TEST_BEFORE_INSTALL
                1
            INSTALL_COMMAND
                ""
            )
endif ()

install(
        DIRECTORY
            ${STAGED_INSTALL_PREFIX}/
        DESTINATION
            .
        USE_SOURCE_PERMISSIONS
        )
