# Makefile for monolith

all: build
.PHONY: all

build:
	@cargo build --locked
.PHONY: build

clean:
	@cargo clean
.PHONY: clean

install:
	@cargo install --force --locked --path .
.PHONY: install

lint:
	@cargo fmt --all --
.PHONY: lint

lint_check:
	@cargo fmt --all -- --check
	@cargo clippy --
.PHONY: lint_check

test: build
	@cargo test --locked
.PHONY: test

uninstall:
	@cargo uninstall
.PHONY: uninstall

update-lock-file:
	@cargo update
.PHONY: clean
