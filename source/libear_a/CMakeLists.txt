set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_IMPLICIT_LINK_LIBRARIES "")
set(CMAKE_CXX_IMPLICIT_LINK_DIRECTORIES "")

add_library(ear_a STATIC
        Array.h
        Storage.h
        Storage.cc
        DynamicLinker.h
        Execution.h
        Executor.h
        Session.h
        State.h
        String.h
)

target_link_libraries(ear_a ${CMAKE_DL_LIBS})
set_target_properties(ear_a PROPERTIES LINKER_LANGUAGE C)
