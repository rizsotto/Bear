add_library(main_a OBJECT
        source/ApplicationLogConfig.cc
        source/ApplicationFromArgs.cc
        )

target_include_directories(main_a PUBLIC include/)
target_link_libraries(main_a PUBLIC result_a)
target_link_libraries(main_a PUBLIC flags_a)
target_link_libraries(main_a PUBLIC fmt::fmt)
target_link_libraries(main_a PUBLIC spdlog::spdlog)
