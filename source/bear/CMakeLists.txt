# Create an executable from the sub projects.
add_executable(bear
        main.cc
        )

target_include_directories(bear PRIVATE ../citnames/include)
target_include_directories(bear PRIVATE ../intercept/include)
target_link_libraries(bear sys_a)
target_link_libraries(bear flags_a)
target_link_libraries(bear fmt::fmt)
target_link_libraries(bear spdlog::spdlog)

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
