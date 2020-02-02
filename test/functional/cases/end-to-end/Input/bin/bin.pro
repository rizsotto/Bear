TEMPLATE = app
TARGET = some_app

CONFIG -= qt x11

INCLUDEPATH += ../lib
LIBS += -L../lib -lsome_lib

HEADERS += boo.h++

SOURCES += boo.c++
SOURCES += far.cxx
