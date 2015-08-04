SRC:=empty.c
OBJ:=empty.o
EXE:=empty

test-in-one: $(SRC)
	cc -o $(EXE) $(SRC)

test-in-two: $(SRC)
	cc -c $(SRC)
	cc -o $(EXE) $(OBJ)

.PHONY: clean
clean:
	rm -f $(OBJ) $(EXE)
