all:
	$(CXX) -DEXPORT="extern \"C\"" -o hello hello.cxx

.PHONY: clean
clean:
	rm -f hello
