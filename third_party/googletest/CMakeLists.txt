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
                SOURCE_DIR
                "${CMAKE_CURRENT_SOURCE_DIR}/googletest"
                CMAKE_CACHE_ARGS
                -DINSTALL_GTEST:BOOL=ON
                -DCMAKE_INSTALL_PREFIX:PATH=${DEPENDENCIES_INSTALL_PREFIX}/googletest
                )
        #        set(googletest_CMAKE_DIR "${DEPENDENCIES_INSTALL_PREFIX}/googletest/${LIBRARY_DIRECTORY_NAME}/cmake/GTest/")
    endif ()
