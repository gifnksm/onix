MAKEFLAGS += --warn-undefined-variables
MAKEFLAGS += --no-builtin-rules
MAKEFLAGS += --no-builtin-variables

SHELL := bash
.SHELLFLAGS := -eu -o pipefail -c
.DELETE_ON_ERROR:
.SECONDARY: # don't remove intermediate files
.SECONDEXPANSION:

.PHONY: default
default: help

CARGO_CROSS_TARGET ?= riscv64imac-unknown-none-elf
CARGO_CROSS_FLAGS ?= \
	-Zbuild-std=core,compiler_builtins,alloc,panic_abort \
	-Zbuild-std-features="compiler-builtins-mem" \
	--target riscv64imac-unknown-none-elf

## Build the project
.PHONY: all
all:

## Print this message
.PHONY: help
help:
	@printf "Available targets:\n\n"
	@awk '/^[a-zA-Z\-_0-9%:\\]+/ { \
		helpMessage = match(lastLine, /^## (.*)/); \
		if (helpMessage) { \
			helpCommand = $$1; \
			helpMessage = substr(lastLine, RSTART + 3, RLENGTH); \
			gsub("\\\\", "", helpCommand); \
			gsub(":+$$", "", helpCommand); \
			printf "  \x1b[32;01m%-24s\x1b[0m %s\n", helpCommand, helpMessage; \
		} \
	} \
	{ \
		if ($$0 !~ /^.PHONY/) { \
			lastLine = $$0 \
		} \
	} \
	' $(MAKEFILE_LIST) | sort -u
	@printf "\n"

.PHONY: FORCE
FORCE:

## Clean the project
.PHONY: clean
clean:
	cargo clean

## Build the project
.PHONY: build
build:
	cargo build -p kernel $(CARGO_CROSS_FLAGS)

## Run the project
.PHONY: run
run:
	cargo run -p kernel $(CARGO_CROSS_FLAGS)

## Test the project
.PHONY: test
test:
	cargo nextest run
