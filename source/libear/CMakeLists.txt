set(CMAKE_CXX_STANDARD 14)
set(CMAKE_CXX_IMPLICIT_LINK_LIBRARIES "")
set(CMAKE_CXX_IMPLICIT_LINK_DIRECTORIES "")
set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -fno-exceptions -fno-rtti")

add_library(ear SHARED
        lib.cc
)
target_link_libraries(ear ear_a)
target_link_libraries(ear ${CMAKE_DL_LIBS})
set_target_properties(ear PROPERTIES LINKER_LANGUAGE C)

if(APPLE)
    set(CMAKE_OSX_ARCHITECTURES "i386;x86_64" CACHE STRING  "Rogue")
    set(CMAKE_MACOSX_RPATH 1)
    set_target_properties(ear PROPERTIES INSTALL_RPATH "@loader_path/${EAR_LIB_PATH}")
endif()

install(TARGETS ear
    LIBRARY DESTINATION ${EAR_LIB_PATH})
