SRC:=empty.c
OBJ:=empty.o
EXE:=empty

test-in-one: $(SRC)
	c++ -o $(EXE) $(SRC)

test-in-two: $(SRC)
	c++ -c $(SRC)
	c++ -o $(EXE) $(OBJ)

.PHONY: clean
clean:
	rm -f $(OBJ) $(EXE)
