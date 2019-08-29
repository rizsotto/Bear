include(FindPythonInterp)
if (PYTHONINTERP_FOUND)
    set(BEAR_PYTHON_EXECUTABLE "python${PYTHON_VERSION_MAJOR}")
else()
    message(FATAL_ERROR "Have not found python interpreter")
endif()

configure_file(${CMAKE_CURRENT_SOURCE_DIR}/main.py.in ${CMAKE_CURRENT_BINARY_DIR}/bear)

include(GNUInstallDirs)
install(PROGRAMS ${CMAKE_CURRENT_BINARY_DIR}/bear
    DESTINATION ${CMAKE_INSTALL_BINDIR})
