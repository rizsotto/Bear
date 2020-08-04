include(FindPythonInterp)
if (PYTHONINTERP_FOUND)
    get_filename_component(BEAR_PYTHON_EXECUTABLE ${PYTHON_EXECUTABLE} NAME)
else ()
    message(FATAL_ERROR "Have not found python interpreter")
endif ()

configure_file(bear.py ${CMAKE_CURRENT_BINARY_DIR}/bear.old)

include(GNUInstallDirs)
install(PROGRAMS ${CMAKE_CURRENT_BINARY_DIR}/bear.old
        DESTINATION ${CMAKE_INSTALL_BINDIR})
