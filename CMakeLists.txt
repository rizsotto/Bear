# This file is distributed under MIT-LICENSE. See COPYING for details.

cmake_minimum_required(VERSION 2.8)

project(bear C)

set(CPACK_PACKAGE_NAME "bear")
set(CPACK_PACKAGE_CONTACT "Laszlo Nagy <rizsotto at gmail dot com>")
set(CPACK_PACKAGE_VENDOR ${CPACK_PACKAGE_CONTACT})
set(CPACK_PACKAGE_VERSION "0.5")
set(CPACK_PACKAGE_DESCRIPTION_SUMMARY "BuildEAR")
set(CPACK_RESOURCE_FILE_LICENSE "${CMAKE_CURRENT_SOURCE_DIR}/COPYING")
set(CPACK_RESOURCE_FILE_README "${CMAKE_CURRENT_SOURCE_DIR}/README.md")
set(CPACK_PACKAGING_INSTALL_PREFIX ${CMAKE_INSTALL_PREFIX})
set(CPACK_GENERATOR "TGZ;RPM;DEB")
set(CPACK_RPM_PACKAGE_RELEASE "1%{?dist}")
set(CPACK_RPM_PACKAGE_LICENSE "MIT")
set(CPACK_RPM_PACKAGE_GROUP "Development/Tools")
set(CPACK_RPM_PACKAGE_URL "http://github.com/rizsotto/Bear")
set(CPACK_RPM_PACKAGE_DESCRIPTION "Bear is a tool to generate compilation database for clang tooling.")
include(CPack)

include(GNUInstallDirs)
install(FILES COPYING README.md
  DESTINATION ${CMAKE_INSTALL_DOCDIR})

add_subdirectory(src)
add_subdirectory(test)
add_subdirectory(doc)
