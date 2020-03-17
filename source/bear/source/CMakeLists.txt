find_package(Python COMPONENTS Interpreter)
if (Python_Interpreter_FOUND)
    if (Python_VERSION_MAJOR GREATER 2)
        set(BEAR_PYTHON_EXECUTABLE "python${Python_VERSION_MAJOR}")
    else()
        # Some systems (e.g. macOS) don't install python2 but only python2.7
        set(BEAR_PYTHON_EXECUTABLE "python${Python_VERSION_MAJOR}.${Python_VERSION_MINOR}")
    endif()
else ()
    message(FATAL_ERROR "Have not found python interpreter")
endif ()

configure_file(bear.py ${CMAKE_CURRENT_BINARY_DIR}/bear)

include(GNUInstallDirs)
install(PROGRAMS ${CMAKE_CURRENT_BINARY_DIR}/bear
        DESTINATION ${CMAKE_INSTALL_BINDIR})
