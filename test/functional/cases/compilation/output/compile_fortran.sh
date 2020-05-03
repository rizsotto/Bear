#!/usr/bin/env sh

# REQUIRES: preload, shell, dynamic-shell, fortran
# RUN: cd %T; %{bear} -vvvv --cdb %t.json -- %{shell} %s %{fortran}
# RUN: assert_compilation %t.json count -eq 1
# RUN: assert_compilation %t.json contains -file compile_fortran.f95 -directory %T -arguments %{fortran} -c -o compile_fortran.o compile_fortran.f95

cat > compile_fortran.f95 << EOF
! Test Program
program first
print *,'This is my first program'
end program first
EOF

$1 -c -o compile_fortran.o compile_fortran.f95
