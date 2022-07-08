
message(STATUS "Looking for fmt dependency")
find_package(fmt 6.1 QUIET CONFIG)
if (fmt_FOUND)
    message(STATUS "Looking for fmt dependency -- found")
    add_custom_target(fmt_dependency)
else ()
    message(STATUS "Looking for fmt dependency -- not found")

    include(ExternalProject)
    ExternalProject_Add(fmt_dependency
            URL
                https://github.com/fmtlib/fmt/archive/9.0.0.tar.gz
            URL_HASH
                MD5=d56c8b0612b049bb1854f07c8b133f3c
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
                -DFMT_INSTALL:BOOL=ON
                -DFMT_TEST:BOOL=OFF
                -DFMT_FUZZ:BOOL=OFF
                -DFMT_DOC:BOOL=OFF
                -DCMAKE_INSTALL_PREFIX:PATH=${DEPENDENCIES_INSTALL_PREFIX}/fmt_dependency
            CMAKE_CACHE_ARGS
                -DCMAKE_BUILD_TYPE:STRING=${CMAKE_BUILD_TYPE}
                -DCMAKE_TOOLCHAIN_FILE:PATH=${CMAKE_TOOLCHAIN_FILE}
                -DCMAKE_FIND_ROOT_PATH:PATH=${CMAKE_FIND_ROOT_PATH}
                -DCMAKE_C_COMPILER:STRING=${CMAKE_C_COMPILER}
                -DCMAKE_C_COMPILER_TARGET:STRING=${CMAKE_C_COMPILER_TARGET}
                -DCMAKE_CXX_COMPILER:STRING=${CMAKE_CXX_COMPILER}
                -DCMAKE_CXX_COMPILER_TARGET:STRING=${CMAKE_CXX_COMPILER_TARGET}
                -DCMAKE_CXX_STANDARD:STRING=${CMAKE_CXX_STANDARD}
                -DCMAKE_CXX_STANDARD_REQUIRED:BOOL=${CMAKE_CXX_STANDARD_REQUIRED}
                -DCMAKE_CXX_EXTENSIONS:BOOL=${CMAKE_CXX_EXTENSIONS}
                -DCMAKE_CXX_FLAGS:STRING=${CMAKE_CXX_FLAGS}
                -DCMAKE_EXE_LINKER_FLAGS:STRING=${CMAKE_EXE_LINKER_FLAGS}
            )

    include(GNUInstallDirs)
    list(APPEND PREFIX_PATH "${DEPENDENCIES_INSTALL_PREFIX}/fmt_dependency")
    set(CMAKE_PREFIX_PATH ${PREFIX_PATH};${CMAKE_PREFIX_PATH}
            CACHE PATH "append fmt library into the search path"
            FORCE)
endif ()
