all: add.wasm nothing.wasm

%.wasm: %.wat
	wat2wasm $< -o $@

.PHONY: clean
clean:
	rm *.wasm
