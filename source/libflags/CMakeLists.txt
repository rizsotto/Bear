add_library(flags_a OBJECT)
target_include_directories(flags_a PUBLIC include/)
target_sources(flags_a
        PRIVATE
            source/Flags.cc
        INTERFACE
            $<TARGET_OBJECTS:flags_a>
        )
target_link_libraries(flags_a PUBLIC result_a fmt::fmt)

if (ENABLE_UNIT_TESTS)
    add_executable(flags_unit_test
            test/FlagsTest.cc
            )

    target_link_libraries(flags_unit_test flags_a)
    target_link_libraries(flags_unit_test PkgConfig::GTest ${CMAKE_THREAD_LIBS_INIT})

    add_test(NAME bear::flags_unit_test COMMAND $<TARGET_FILE:flags_unit_test>)
endif ()