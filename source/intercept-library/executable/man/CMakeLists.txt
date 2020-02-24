# Markdown file is the source to the man file. Please modify that and generate
# the man file from it with pandoc.
#
#   $ pandoc -s -t man er.1.md -o er.1
#
# This is not automated, because pandoc has big dependencies on different OS
# distributions and packaging would require to install those. Which might be
# too much effort to generate a single text file.

include(GNUInstallDirs)
install(FILES ${CMAKE_CURRENT_SOURCE_DIR}/er.1
        DESTINATION ${CMAKE_INSTALL_MANDIR}/man1)
