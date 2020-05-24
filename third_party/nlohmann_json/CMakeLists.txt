message(STATUS "Looking for nlohman_json dependency")
find_package(nlohmann_json 3.7.3 QUIET)
if (nlohmann_json_FOUND)
    message(STATUS "Looking for nlohman_json dependency -- found")
    add_custom_target(nlohmann_json_dependency)
else ()
    message(STATUS "Looking for nlohman_json dependency -- not found")

    include(ExternalProject)
    ExternalProject_Add(nlohmann_json_dependency
            SOURCE_DIR
            "${CMAKE_CURRENT_SOURCE_DIR}/nlohmann_json"
            CMAKE_CACHE_ARGS
            -DJSON_Install:BOOL=ON
            -DJSON_BuildTests:BOOL=OFF
            -DCMAKE_INSTALL_PREFIX:PATH=${DEPENDENCIES_INSTALL_PREFIX}/nlohmann_json
            )
    #    set(json_CMAKE_DIR "${DEPENDENCIES_INSTALL_PREFIX}/nlohmann_json/${LIBRARY_DIRECTORY_NAME}/cmake/nlohmann_json/")
endif ()
