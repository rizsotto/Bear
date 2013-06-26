message(STATUS "Looking for pandoc")
find_program(PANDOC_COMMAND pandoc)

if (PANDOC_COMMAND)
  message(STATUS "Looking for pandoc -- found")
  add_custom_target(generate_doc ALL
    COMMAND ${PANDOC_COMMAND} -s -w man ${CMAKE_CURRENT_SOURCE_DIR}/bear.1.md -o bear.1
    SOURCES ${CMAKE_CURRENT_SOURCE_DIR}/bear.1.md)

  include(GNUInstallDirs)
  install(FILES ${CMAKE_CURRENT_BINARY_DIR}/bear.1
    DESTINATION ${CMAKE_INSTALL_MANDIR}/man1)
else()
  message(STATUS "Looking for pandoc -- not found")
  message(STATUS "- skip man page generation")
endif()
