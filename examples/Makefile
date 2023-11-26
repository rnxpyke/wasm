all: add.wasm nothing.wasm greet.wasm

%.wasm: %.wat
	wat2wasm $< -o $@

.PHONY: clean
clean:
	rm *.wasm
