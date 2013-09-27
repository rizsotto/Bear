find_package(PythonInterp)

set(BEAR_EXE ${CMAKE_CURRENT_BINARY_DIR}/../../src/bear)

if(PYTHONINTERP_FOUND)
  add_custom_target(run_config_failure_test
    COMMAND python ${CMAKE_CURRENT_SOURCE_DIR}/run.py --sut ${BEAR_EXE} --test-cases ${CMAKE_CURRENT_SOURCE_DIR}/tests
    COMMENT "Running config failure tests")

  add_dependencies(run_config_failure_test ear)
  add_dependencies(run_config_failure_test bear)

  add_test(NAME run_config_failure_test
    COMMAND make run_config_failure_test)
endif()
