
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
                https://github.com/gabime/spdlog/archive/v1.7.0.tar.gz
            URL_HASH
                MD5=2e1c0c45920a3536366d2232963bc24a
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
            )

    include(GNUInstallDirs)
    list(APPEND PREFIX_PATH "${DEPENDENCIES_INSTALL_PREFIX}/spdlog_dependency")
    set(CMAKE_PREFIX_PATH ${CMAKE_PREFIX_PATH};${PREFIX_PATH}
            CACHE PATH "append spdlog library into the search path"
            FORCE)
endif ()
