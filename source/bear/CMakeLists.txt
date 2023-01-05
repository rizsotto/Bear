# Create a static library, which is used for unit tests and the final shared library.
add_library(bear_a OBJECT)
target_include_directories(bear_a PUBLIC source/ ../citnames/include/ ../intercept/include/)
target_sources(bear_a
        PRIVATE
            source/Application.cc
        INTERFACE
            $<TARGET_OBJECTS:bear_a>
        )
target_link_libraries(bear_a PUBLIC
        main_a
        sys_a
        flags_a
        fmt::fmt
        citnames_a
        intercept_a
        spdlog::spdlog)

# Create an executable from the sub projects.
add_executable(bear
        main.cc
        )
target_link_libraries(bear
        bear_a)

include(GNUInstallDirs)
install(TARGETS bear
        RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR})

# Markdown file is the source to the man file. Please modify that and generate
# the man file from it with pandoc.
#
#   $ pandoc -s -t man bear.1.md -o bear.1
#
# This is not automated, because pandoc has big dependencies on different OS
# distributions and packaging would require to install those. Which might be
# too much effort to generate a single text file.

install(FILES man/bear.1
        DESTINATION ${CMAKE_INSTALL_MANDIR}/man1)
