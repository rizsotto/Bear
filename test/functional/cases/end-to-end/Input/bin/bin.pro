TEMPLATE = app
TARGET = some_app

INCLUDEPATH += ../lib
LIBS += -L../lib -lsome_lib

HEADERS += boo.h++

SOURCES += boo.c++
SOURCES += far.cxx
