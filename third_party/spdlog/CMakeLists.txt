
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
                https://github.com/gabime/spdlog/archive/v1.6.0.tar.gz
            URL_HASH
                MD5=34f148435461c4747d5d93bacff3dabc
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
                -Dfmt_DIR:PATH=${fmt_DIR}
                -DSPDLOG_INSTALL:BOOL=ON
                -DSPDLOG_NO_EXCEPTIONS:BOOL=ON
                -DSPDLOG_BUILD_TESTS:BOOL=OFF
                -DSPDLOG_BUILD_EXAMPLE:BOOL=OFF
                -DCMAKE_INSTALL_PREFIX:PATH=${DEPENDENCIES_INSTALL_PREFIX}/spdlog_dependency
            )

    include(GNUInstallDirs)
    set(spdlog_DIR "${DEPENDENCIES_INSTALL_PREFIX}/spdlog_dependency/${CMAKE_INSTALL_LIBDIR}/cmake/spdlog/"
            CACHE PATH "Path to internally built spdlogConfig.cmake"
            FORCE
            )
endif ()
