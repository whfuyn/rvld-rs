OUT_DIR="out"

default: run

build: $(OUT_DIR)/main.o $(OUT_DIR)/rvld

run: build
	@$(OUT_DIR)/rvld $(OUT_DIR)/main.o

$(OUT_DIR)/rvld:
	@cargo b
	@cp target/riscv64gc-unknown-linux-gnu/debug/rvld $(OUT_DIR)/rvld

$(OUT_DIR)/main.o: main.c
	@riscv64-linux-gnu-gcc -c -o $(OUT_DIR)/main.o -xc main.c

clean:
	@rm $(OUT_DIR)/*

test:
	echo "$(wildcard src/*.rs)"

.PHONY: default clean build run