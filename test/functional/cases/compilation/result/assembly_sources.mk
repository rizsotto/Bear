#!/usr/bin/env make

# REQUIRES: preload, make, dynamic-make
# RUN: cd %T; %{bear} -vvvv --cdb %t.json -- %{make} -C %T -f %s
# RUN: assert_compilation %t.json count -eq 2
# RUN: assert_compilation %t.json contains -file main.c -directory %T -arguments %{c_compiler} -S -o main.s main.c
# RUN: assert_compilation %t.json contains -file main.s -directory %T -arguments %{c_compiler} -c -o main.o main.s

main: main.o
	$(CC) $< -o $@

main.s: main.c
	$(CC) -S $< -o $@

main.o: main.s
	$(CC) -c $< -o $@

main.c:
	echo "int main() { return 0; }" > $@
