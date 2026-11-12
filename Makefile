# Makefile for monolith

all: build build-gui
.PHONY: all

build:
	@cargo build --locked
.PHONY: build

build-gui:
	@cargo build --locked --bin monolith-gui --features="gui"
.PHONY: build_gui

clean:
	@cargo clean
.PHONY: clean

format:
	@cargo fmt --all --
.PHONY: format

format-check:
	@cargo fmt --all -- --check
.PHONY: format

install:
	@cargo install --force --locked --path .
.PHONY: install

lint:
	@cargo clippy --fix --allow-dirty --allow-staged
# 	@cargo fix --allow-dirty --allow-staged
.PHONY: lint

lint-check:
	@cargo clippy --all-targets -- -D warnings
	@cargo clippy --all-targets --no-default-features --features cli -- -D warnings
.PHONY: lint-check

test: build
	@cargo test --locked --no-fail-fast
.PHONY: test

uninstall:
	@cargo uninstall
.PHONY: uninstall

update-lock-file:
	@cargo update
.PHONY: clean
