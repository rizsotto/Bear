# This file is distributed under MIT-LICENSE. See COPYING for details.

add_custom_target(end-to-end
  COMMAND echo "TBD"
  COMMENT "Running module tests")

add_dependencies(end-to-end ear)
add_dependencies(end-to-end bear)
