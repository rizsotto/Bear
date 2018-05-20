set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_IMPLICIT_LINK_LIBRARIES "")
set(CMAKE_CXX_IMPLICIT_LINK_DIRECTORIES "")

add_library(ear_a STATIC
        Array.h
        DynamicLinker.h
        DynamicLinker.cc
        Executor.h
        Interface.h
        Session.h
        State.h
        State.cc
        Storage.h
        Storage.cc
        String.h
)

set_property(TARGET ear_a PROPERTY POSITION_INDEPENDENT_CODE 1)
target_link_libraries(ear_a ${CMAKE_DL_LIBS})
set_target_properties(ear_a PROPERTIES LINKER_LANGUAGE C)
