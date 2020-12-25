message(STATUS "Looking for sqlite dependency")

find_package(PkgConfig REQUIRED)
pkg_check_modules(sqlite sqlite3>=3.14)

if (sqlite_FOUND)
    message(STATUS "Looking for sqlite dependency -- found")
    add_custom_target(sqlite_dependency)
else ()
    message(FATAL_ERROR "Looking for sqlite dependency -- not found")
endif ()
