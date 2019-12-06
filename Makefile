.PHONY: all build install run test lint

all: test build

build:
	@cargo build --locked

install:
	@cargo install --force --locked --path .

test:
	@cargo test --locked
	@cargo fmt --all -- --check

lint:
	@cargo fmt --all --
