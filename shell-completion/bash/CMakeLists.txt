find_package(bash-completion)
if (NOT BASH_COMPLETION_FOUND OR NOT CMAKE_INSTALL_PREFIX_INITIALIZED_TO_DEFAULT)
    # Fallback default dir
    set(BASH_COMPLETION_COMPLETIONSDIR
        "${CMAKE_INSTALL_DATADIR}/bash-completion/completions/")
endif()

install(
    FILES "bear"
    DESTINATION "${BASH_COMPLETION_COMPLETIONSDIR}"
    COMPONENT shell-completion
    )
