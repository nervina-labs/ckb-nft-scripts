build:
	cd contracts && cargo fmt
	capsule build

build-release:
	capsule build --release

test:
	cd contracts && cargo fmt
	cp libs/* build/debug/
	capsule test

test-release:
	cp libs/* build/release/
	capsule test --release

clean:
	rm -rf build/debug

clean-release:
	rm -rf build/release

.PHONY: build test clean