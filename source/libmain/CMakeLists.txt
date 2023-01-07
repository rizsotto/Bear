add_library(main_a OBJECT)
target_include_directories(main_a PUBLIC include/)
target_sources(main_a
        PRIVATE
            source/ApplicationLogConfig.cc
            source/ApplicationFromArgs.cc
            source/SubcommandFromArgs.cc
        INTERFACE
            $<TARGET_OBJECTS:main_a>
        )
target_link_libraries(main_a PUBLIC
        result_a
        flags_a
        fmt::fmt
        spdlog::spdlog)
