all: check build test

todos:
	rg -g '!Makefile' -i todo 

check:
	cargo check

build:
	cargo build

test:
	cargo test

docs:
	
release: release-bump all docs
	git commit -am "Bump to version $$(cargo read-manifest | jq .version)"
	git tag v$$(cargo read-manifest | jq -r .version)

release-bump:
	cargo bump

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
