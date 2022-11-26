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
            URL
                https://github.com/google/googletest/archive/release-1.12.1.tar.gz
            URL_HASH
                MD5=e82199374acdfda3f425331028eb4e2a
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
                -DINSTALL_GTEST:BOOL=ON
                -DCMAKE_INSTALL_PREFIX:PATH=${DEPENDENCIES_INSTALL_PREFIX}/googletest_dependency
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
            )

    include(GNUInstallDirs)
    list(APPEND PREFIX_PATH "${DEPENDENCIES_INSTALL_PREFIX}/googletest_dependency")
    set(CMAKE_PREFIX_PATH ${PREFIX_PATH};${CMAKE_PREFIX_PATH}
            CACHE PATH "append googletest library into the search path"
            FORCE)
endif ()
