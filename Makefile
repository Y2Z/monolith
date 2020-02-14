#!/usr/bin/make -f

all: test
.PHONY: all

build:
	@cargo build --locked
.PHONY: build

install:
	@cargo install --force --locked --path .
.PHONY: install

test: build
	@cargo test --locked
	@cargo fmt --all -- --check
.PHONY: test

lint:
	@cargo fmt --all --
.PHONY: lint
