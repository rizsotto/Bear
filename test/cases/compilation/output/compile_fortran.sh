#!/usr/bin/env sh

# REQUIRES: shell, fortran
# RUN: cd %T; env FC=%{fortran} %{bear} --verbose --output-compile %t.json -- %{shell} %s
# RUN: assert_compilation %t.json count -eq 1
# RUN: assert_compilation %t.json contains -file %T/compile_fortran.f95 -directory %T -arguments %{fortran} -c -o compile_fortran.o compile_fortran.f95

cat > compile_fortran.f95 << EOF
! Test Program
program first
print *,'This is my first program'
end program first
EOF

$FC -c -o compile_fortran.o compile_fortran.f95
