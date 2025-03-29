# Makefile for monolith

all: build build_gui
.PHONY: all

build:
	@cargo build --locked
.PHONY: build

build_gui:
	@cargo build --locked --bin monolith-gui --features="gui"
.PHONY: build_gui

clean:
	@cargo clean
.PHONY: clean

format:
	@cargo fmt --all --
.PHONY: format

format_check:
	@cargo fmt --all -- --check
.PHONY: format

install:
	@cargo install --force --locked --path .
.PHONY: install

lint:
	@cargo clippy --fix --allow-dirty --allow-staged
.PHONY: lint

lint_check:
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
