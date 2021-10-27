fmt:
	cd contracts && cargo fmt
	cargo fmt
build:
	fmt
	capsule build

build-release:
	capsule build --release

test:
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