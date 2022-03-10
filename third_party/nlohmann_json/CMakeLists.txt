message(STATUS "Looking for nlohman_json dependency")
find_package(nlohmann_json 3.7.3 QUIET)
if (nlohmann_json_FOUND)
    message(STATUS "Looking for nlohman_json dependency -- found")
    add_custom_target(nlohmann_json_dependency)
else ()
    message(STATUS "Looking for nlohman_json dependency -- not found")

    include(ExternalProject)
    ExternalProject_Add(nlohmann_json_dependency
            URL
                https://github.com/nlohmann/json/archive/v3.10.5.tar.gz
            URL_HASH
                MD5=5b946f7d892fa55eabec45e76a20286b
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
                -DJSON_Install:BOOL=ON
                -DJSON_BuildTests:BOOL=OFF
                -DCMAKE_INSTALL_PREFIX:PATH=${DEPENDENCIES_INSTALL_PREFIX}/nlohmann_json_dependency
            )

    include(GNUInstallDirs)
    list(APPEND PREFIX_PATH "${DEPENDENCIES_INSTALL_PREFIX}/nlohmann_json_dependency")
    set(CMAKE_PREFIX_PATH ${PREFIX_PATH};${CMAKE_PREFIX_PATH}
            CACHE PATH "append JSON library into the search path"
            FORCE)
endif ()
