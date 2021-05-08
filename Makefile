ENVIRONMENT := debug

all: 
	capsule build

simulators: simulator/natives-script-utils simulator/natives-issuer-type simulator/natives-class-type simulator/natives-nft-type 
	mkdir -p build/$(ENVIRONMENT)
	cp target/$(ENVIRONMENT)/ckb-script-utils-sim build/$(ENVIRONMENT)/ckb-script-utils-sim
	cp target/$(ENVIRONMENT)/ckb-issuer-type-sim build/$(ENVIRONMENT)/ckb-issuer-type-sim
	cp target/$(ENVIRONMENT)/ckb-class-type-sim build/$(ENVIRONMENT)/ckb-class-type-sim
	cp target/$(ENVIRONMENT)/ckb-nft-type-sim build/$(ENVIRONMENT)/ckb-nft-type-sim

simulator/natives-issuer-type:
	CARGO_INCREMENTAL=0 RUSTFLAGS="-Zprofile -Ccodegen-units=1 -Copt-level=0 -Clink-dead-code -Coverflow-checks=off -Zpanic_abort_tests -Cpanic=abort" RUSTDOCFLAGS="-Cpanic=abort" cargo build -p natives-issuer-type

simulator/natives-class-type:
	CARGO_INCREMENTAL=0 RUSTFLAGS="-Zprofile -Ccodegen-units=1 -Copt-level=0 -Clink-dead-code -Coverflow-checks=off -Zpanic_abort_tests -Cpanic=abort" RUSTDOCFLAGS="-Cpanic=abort" cargo build -p natives-class-type

simulator/natives-nft-type:
	CARGO_INCREMENTAL=0 RUSTFLAGS="-Zprofile -Ccodegen-units=1 -Copt-level=0 -Clink-dead-code -Coverflow-checks=off -Zpanic_abort_tests -Cpanic=abort" RUSTDOCFLAGS="-Cpanic=abort" cargo build -p natives-nft-type

simulator/natives-script-utils:
	CARGO_INCREMENTAL=0 RUSTFLAGS="-Zprofile -Ccodegen-units=1 -Copt-level=0 -Clink-dead-code -Coverflow-checks=off -Zpanic_abort_tests -Cpanic=abort" RUSTDOCFLAGS="-Cpanic=abort" cargo build -p natives-script-utils

test: simulators
	cargo test -p tests
	./scripts/run_sim_tests.sh $(ENVIRONMENT)

coverage: test
	zip -0 build/$(ENVIRONMENT)/ccov.zip `find . \( -name "ckb-time-scripts-sim*.gc*" \) -print`
	grcov build/$(ENVIRONMENT)/ccov.zip -s . -t lcov --llvm --branch --ignore-not-existing --ignore "/*" -o build/$(ENVIRONMENT)/lcov.info
	genhtml -o build/$(ENVIRONMENT)/coverage/ --rc lcov_branch_coverage=1 --show-details --highlight --ignore-errors source --legend build/$(ENVIRONMENT)/lcov.info

clean:	
	cargo clean
	rm -rf build/$(ENVIRONMENT)

.PHONY: all simulators test coverage clean