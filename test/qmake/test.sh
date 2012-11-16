#!/bin/sh

# fix fedora path
export PATH=$PATH:/usr/lib64/qt4/bin

qmake test.pro && $@ -- make
