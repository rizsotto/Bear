include(GNUInstallDirs)
install(FILES "bear"
    DESTINATION "${CMAKE_INSTALL_DATADIR}/bash-completion/completions/"
    COMPONENT shell-completion
)
