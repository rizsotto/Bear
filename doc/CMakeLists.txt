# This file is distributed under MIT-LICENSE. See COPYING for details.

add_custom_target(generate_doc ALL
  COMMAND pandoc -s -w man ${CMAKE_CURRENT_SOURCE_DIR}/bear.1.md -o bear.1
  SOURCES ${CMAKE_CURRENT_SOURCE_DIR}/bear.1.md)

include(GNUInstallDirs)
install(FILES ${CMAKE_CURRENT_BINARY_DIR}/bear.1
  DESTINATION ${CMAKE_INSTALL_MANDIR}/man1)
