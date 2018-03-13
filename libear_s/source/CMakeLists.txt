add_library(ear SHARED
        ./ear.cc
        ./Array.h
        ./String.h
        ./Environment.h
        ./DynamicLinker.h
        ./Executor.h)
target_link_libraries(ear ${CMAKE_DL_LIBS})
set_target_properties(ear PROPERTIES LINKER_LANGUAGE C)

if(APPLE)
    set(CMAKE_OSX_ARCHITECTURES "i386;x86_64" CACHE STRING  "Rogue")
    set(CMAKE_MACOSX_RPATH 1)
    set_target_properties(ear PROPERTIES INSTALL_RPATH "@loader_path/${EAR_LIB_PATH}")
endif()

install(TARGETS ear
    LIBRARY DESTINATION ${EAR_LIB_PATH})
