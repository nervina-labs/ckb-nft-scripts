build:
	cargo fmt
	capsule build

build-release:
	cargo fmt
	capsule build --release

test:
	cargo fmt
	capsule test

test-release:
	capsule test --release

clean:
	rm -rf build/debug

clean-release:
	rm -rf build/release

.PHONY: build test clean
