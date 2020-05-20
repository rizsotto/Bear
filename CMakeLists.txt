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

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

set_property(DIRECTORY PROPERTY EP_BASE ${CMAKE_BINARY_DIR}/subprojects)
set(STAGED_INSTALL_PREFIX ${CMAKE_BINARY_DIR}/stage)
set(DEPENDENCIES_INSTALL_PREFIX ${CMAKE_BINARY_DIR}/subprojects/Install)

add_subdirectory(third_party)

include(ExternalProject)
ExternalProject_Add(Bear
        SOURCE_DIR
            "${CMAKE_CURRENT_SOURCE_DIR}/source"
        DEPENDS
            # TODO: googletest_dependency shall be not here if ENABLE_UNIT_TESTS is OFF
            nlohmann_json_dependency fmt_dependency spdlog_dependency grpc_dependency googletest_dependency
        CMAKE_ARGS
            -DCMAKE_CXX_STANDARD=${CMAKE_CXX_STANDARD}
            -DCMAKE_CXX_STANDARD_REQUIRED=${CMAKE_CXX_STANDARD_REQUIRED}
            -DCMAKE_CXX_EXTENSIONS=${CMAKE_CXX_EXTENSIONS}
            -DCMAKE_INSTALL_PREFIX:PATH=${STAGED_INSTALL_PREFIX}
        CMAKE_CACHE_ARGS
            -DCMAKE_CXX_FLAGS:STRING=${CMAKE_CXX_FLAGS}
            -DCMAKE_PREFIX_PATH:PATH=${CMAKE_PREFIX_PATH}
            -DENABLE_UNIT_TESTS:BOOL=${ENABLE_UNIT_TESTS}
        BUILD_ALWAYS
            1
        TEST_BEFORE_INSTALL
            1
        TEST_COMMAND
            ctest # or `ctest -T memcheck`
        )

install(
        DIRECTORY
            ${STAGED_INSTALL_PREFIX}/
        DESTINATION
            .
        USE_SOURCE_PERMISSIONS
)

if (ENABLE_FUNC_TESTS)
    include(GNUInstallDirs)
    ExternalProject_Add(BearTest
            SOURCE_DIR
                "${CMAKE_CURRENT_SOURCE_DIR}/test"
            DEPENDS
                Bear
            CMAKE_CACHE_ARGS
                # TODO: pass the prefix only
                -D_BEAR_BIN_DIR:PATH=${STAGED_INSTALL_PREFIX}/${CMAKE_INSTALL_BINDIR}
                -D_BEAR_LIB_DIR:PATH=${STAGED_INSTALL_PREFIX}/${CMAKE_INSTALL_LIBDIR}
                -D_BEAR_LIBEXEC_DIR:PATH=${STAGED_INSTALL_PREFIX}/${CMAKE_INSTALL_LIBEXECDIR}
            TEST_BEFORE_INSTALL
                1
            INSTALL_COMMAND
                ""
            )
endif ()
