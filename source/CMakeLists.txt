cmake_minimum_required(VERSION 3.1...3.16)
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


project(bear
        VERSION 3.0.0
        DESCRIPTION "Bear is a tool to generate compilation database for clang tooling."
        LANGUAGES C CXX)

find_package(Threads REQUIRED)
find_package(nlohmann_json REQUIRED)
find_package(fmt REQUIRED)
find_package(spdlog REQUIRED)
find_package(GTest REQUIRED NO_CMAKE_PATH)
find_package(Protobuf REQUIRED NO_CMAKE_PATH)
find_package(gRPC REQUIRED NO_CMAKE_PATH)

include(CheckIncludeFile)
check_include_file(spawn.h HAVE_SPAWN_H)
check_include_file(unistd.h HAVE_UNISTD_H)
check_include_file(sys/wait.h HAVE_WAIT_H)
check_include_file(spawn.h HAVE_SPAWN_H)
check_include_file(dlfcn.h HAVE_DLFCN_H)
check_include_file(errno.h HAVE_ERRNO_H)
check_include_file(unistd.h HAVE_UNISTD_H)
check_include_file(sys/utsname.h HAVE_SYS_UTSNAME_H)
include(CheckSymbolExists)
set(CMAKE_REQUIRED_LIBRARIES ${CMAKE_DL_LIBS})
set(CMAKE_REQUIRED_FLAGS -D_GNU_SOURCE)
check_symbol_exists(_NSGetEnviron "crt_externs.h" HAVE_NSGETENVIRON)
check_symbol_exists(dlsym "dlfcn.h" HAVE_DLSYM)
check_symbol_exists(RTLD_NEXT "dlfcn.h" HAVE_RTLD_NEXT)
check_symbol_exists(EACCES "errno.h" HAVE_EACCES)
check_symbol_exists(ENOENT "errno.h" HAVE_ENOENT)
check_symbol_exists(uname "sys/utsname.h" HAVE_UNAME)
check_symbol_exists(confstr "unistd.h" HAVE_CONFSTR)
check_symbol_exists(_CS_PATH "unistd.h" HAVE_CS_PATH)
check_symbol_exists(_CS_GNU_LIBC_VERSION "unistd.h" HAVE_CS_GNU_LIBC_VERSION)
check_symbol_exists(_CS_GNU_LIBPTHREAD_VERSION "unistd.h" HAVE_CS_GNU_LIBPTHREAD_VERSION)

configure_file(config.h.in ${CMAKE_CURRENT_BINARY_DIR}/config.h)
include_directories(${CMAKE_CURRENT_BINARY_DIR})

include(CTest)
enable_testing()

add_subdirectory(bear)
add_subdirectory(libresult)
add_subdirectory(libflags)
add_subdirectory(intercept)
add_subdirectory(intercept-wrapper)
add_subdirectory(intercept-library)

include(GNUInstallDirs)
install(FILES ../COPYING ../README.md ../ChangeLog.md
        DESTINATION ${CMAKE_INSTALL_DOCDIR})

set(CPACK_PACKAGE_NAME "bear")
set(CPACK_PACKAGE_CONTACT "László Nagy")
set(CPACK_PACKAGE_VENDOR ${CPACK_PACKAGE_CONTACT})
set(CPACK_PACKAGE_VERSION ${CMAKE_PROJECT_VERSION})
set(CPACK_PACKAGE_DESCRIPTION_SUMMARY "BuildEAR")
set(CPACK_RESOURCE_FILE_LICENSE "${CMAKE_CURRENT_SOURCE_DIR}/../COPYING")
set(CPACK_RESOURCE_FILE_README "${CMAKE_CURRENT_SOURCE_DIR}/../README.md")
set(CPACK_PACKAGING_INSTALL_PREFIX ${CMAKE_INSTALL_PREFIX})
set(CPACK_RPM_PACKAGE_RELEASE "1%{?dist}")
set(CPACK_RPM_PACKAGE_LICENSE "GPLv3")
set(CPACK_RPM_PACKAGE_GROUP "Development/Tools")
set(CPACK_RPM_PACKAGE_URL "http://github.com/rizsotto/Bear")
set(CPACK_RPM_PACKAGE_DESCRIPTION "Bear is a tool to generate compilation database for clang tooling.")
set(CPACK_RPM_EXCLUDE_FROM_AUTO_FILELIST_ADDITION
        "${CMAKE_INSTALL_MANDIR}"
        "${CMAKE_INSTALL_MANDIR}/man1")
set(CPACK_DEBIAN_PACKAGE_SHLIBDEPS ON)
include(CPack)
