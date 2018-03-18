configure_file(main.py.in ${CMAKE_CURRENT_BINARY_DIR}/bear)

include(GNUInstallDirs)
install(PROGRAMS ${CMAKE_CURRENT_BINARY_DIR}/bear
    DESTINATION ${CMAKE_INSTALL_BINDIR})
