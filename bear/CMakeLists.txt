include(FindPythonInterp)
if (PYTHONINTERP_FOUND)
    if (PYTHON_VERSION_MAJOR GREATER 2)
        set(BEAR_PYTHON_EXECUTABLE "python${PYTHON_VERSION_MAJOR}")
    else()
        # Some systems (e.g. macOS) don't install python2 but only python2.7
        set(BEAR_PYTHON_EXECUTABLE "python${PYTHON_VERSION_MAJOR}.${PYTHON_VERSION_MINOR}")
    endif()
else()
    message(FATAL_ERROR "Have not found python interpreter")
endif()

configure_file(${CMAKE_CURRENT_SOURCE_DIR}/bear.py ${CMAKE_CURRENT_BINARY_DIR}/bear)

include(GNUInstallDirs)
install(PROGRAMS ${CMAKE_CURRENT_BINARY_DIR}/bear
    DESTINATION ${CMAKE_INSTALL_BINDIR})
