.PHONY: all build install run test lint

all: test build

build:
	@cargo build

install:
	@cargo install --force --path .

test:
	@cargo test
	@cargo fmt --all -- --check

lint:
	@cargo fmt --all --
