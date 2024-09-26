
message(STATUS "Looking for spdlog dependency")
find_package(spdlog 1.5.0 QUIET CONFIG)
if (spdlog_FOUND)
    message(STATUS "Looking for spdlog dependency -- found")
    add_custom_target(spdlog_dependency)
else ()
    message(STATUS "Looking for spdlog dependency -- not found")

    include(ExternalProject)
    ExternalProject_Add(spdlog_dependency
            URL
                https://github.com/gabime/spdlog/archive/v1.14.1.tar.gz
            URL_HASH
                MD5=f2c3f15c20e67b261836ff7bfda302cf
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
            DEPENDS
                fmt_dependency
            CMAKE_CACHE_ARGS
                -DSPDLOG_FMT_EXTERNAL:BOOL=ON
                -DSPDLOG_INSTALL:BOOL=ON
                -DSPDLOG_NO_EXCEPTIONS:BOOL=ON
                -DSPDLOG_BUILD_TESTS:BOOL=OFF
                -DSPDLOG_BUILD_EXAMPLE:BOOL=OFF
                -DCMAKE_PREFIX_PATH:PATH=${CMAKE_PREFIX_PATH}
                -DCMAKE_INSTALL_PREFIX:PATH=${DEPENDENCIES_INSTALL_PREFIX}/spdlog_dependency
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
    list(APPEND PREFIX_PATH "${DEPENDENCIES_INSTALL_PREFIX}/spdlog_dependency")
    set(CMAKE_PREFIX_PATH ${PREFIX_PATH};${CMAKE_PREFIX_PATH}
            CACHE PATH "append spdlog library into the search path"
            FORCE)
endif ()
