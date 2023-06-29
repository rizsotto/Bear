#!/usr/bin/env make

# REQUIRES: make, shell
# RUN: mkdir -p %T/make
# RUN: %{make} -C %T -f %s clean
# RUN: %{shell} -c "PATH=%T:$PATH %{bear} --verbose --output %t.json -- %{make} -C %T -f %s"
# RUN: assert_compilation %t.json count -eq 2
# RUN: assert_compilation %t.json contains -file %T/bug439.c -directory %T -arguments %{c_compiler} -S -o bug439.s bug439.c
# RUN: assert_compilation %t.json contains -file %T/bug439.s -directory %T -arguments %{c_compiler} -c -o bug439.o bug439.s

bug439: bug439.o
	$(CC) $< -o $@

bug439.s: bug439.c
	$(CC) -S $< -o $@

bug439.o: bug439.s
	$(CC) -c $< -o $@

bug439.c:
	echo "int main() { return 0; }" > $@

clean:
	rm -f bug439 bug439.o bug439.s bug439.c
