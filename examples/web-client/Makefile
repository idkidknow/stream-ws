.PHONY: build build-wasm deps dev

build: deps
	cd web && npm run build

build-wasm:
	wasm-pack build wasm

deps: build-wasm
	cd web && npm install

dev: deps
	cd web && npm run dev
