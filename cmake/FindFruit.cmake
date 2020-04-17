find_path(Fruit_INCLUDES fruit/fruit.h
        HINTS (
            ${FRUIT_INSTALLED_DIR}
            /usr
            /usr/local
            )
        PATH_SUFFIXES include
        )

find_library(Fruit_LIBRARIES
        NAMES fruit
        HINTS (
            ${FRUIT_INSTALLED_DIR}
            /usr
            /usr/local
            )
        PATH_SUFFIXES lib lib64
        )

include(FindPackageHandleStandardArgs)
find_package_handle_standard_args(Fruit DEFAULT_MSG Fruit_LIBRARIES Fruit_INCLUDES)

message(STATUS "Fruit include directory ... ${Fruit_INCLUDES}")
message(STATUS "Fruit library directory ... ${Fruit_LIBRARIES}")
