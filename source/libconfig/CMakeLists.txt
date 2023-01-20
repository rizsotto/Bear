add_library(config_a OBJECT)
target_include_directories(config_a PUBLIC include/)
target_sources(config_a
        PRIVATE
            source/Configuration.cc
            source/Citnames-config.cc
            source/Intercept-config.cc
        INTERFACE
            $<TARGET_OBJECTS:config_a>
        )
target_link_libraries(config_a PUBLIC
        result_a
        flags_a
        sys_a
        fmt::fmt
        spdlog::spdlog
        nlohmann_json::nlohmann_json)
target_compile_options(config_a PRIVATE -fexceptions)
