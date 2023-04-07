#!/usr/bin/env make

# REQUIRES: make
# RUN: %{make} -C %T -f %s clean
# RUN: %{bear} --verbose --output-compile %t.json -- %{make} -C %T -f %s
# RUN: assert_compilation %t.json count -eq 2
# RUN: assert_compilation %t.json contains -file %T/main.c -directory %T -arguments %{c_compiler} -S -o main.s main.c
# RUN: assert_compilation %t.json contains -file %T/main.s -directory %T -arguments %{c_compiler} -c -o main.o main.s

main: main.o
	$(CC) $< -o $@

main.s: main.c
	$(CC) -S $< -o $@

main.o: main.s
	$(CC) -c $< -o $@

main.c:
	echo "int main() { return 0; }" > $@

clean:
	rm -f main main.o main.s main.c
