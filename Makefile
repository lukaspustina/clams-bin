all: check build test use_case_tests

todos:
	rg -g '!Makefile' -i todo 

check:
	cargo check

build:
	cargo build

test:
	cargo test

use_case_tests: use_cases
	@echo Running Use Case Tests
	@make -C $<

docs:
	
release: release-bump all docs
	git commit -am "Bump to version $$(cargo read-manifest | jq .version)"
	git tag v$$(cargo read-manifest | jq -r .version)

release-bump:
	cargo bump


install:
	cargo install --force

uninstall:
	cargo uninstall

clippy:
	rustup run nightly cargo clippy

fmt:
	rustup run nightly cargo fmt

duplicate_libs:
	cargo tree -d

_update-clippy_n_fmt:
	rustup update
	rustup run nightly cargo install clippy --force
	rustup component add rustfmt-preview --toolchain=nightly

_cargo_install:
	cargo install -f cargo-tree
	cargo install -f cargo-bump
