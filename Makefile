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

CARGO_BUILD_FLAGS ?= \
	-Zbuild-std=core,compiler_builtins,alloc,panic_abort \
	-Zbuild-std-features="compiler-builtins-mem"

CARGO_CROSS_TARGET ?= riscv64imac-unknown-none-elf
CARGO_CROSS_FLAGS ?= \
	--target riscv64imac-unknown-none-elf

CARGO_PROFILE_FLAGS ?=
ifdef RELEASE
	CARGO_PROFILE_FLAGS += --release
endif

## Build the project
.PHONY: all
all: build build-native

## Tidy the project
.PHONY: tidy
tidy: clippy clippy-native

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
	cargo build $(CARGO_BUILD_FLAGS) $(CARGO_CROSS_FLAGS) $(CARGO_PROFILE_FLAGS)

## Build the project for native architecture
.PHONY: build-native
build-native:
	cargo build $(CARGO_BUILD_FLAGS) $(CARGO_PROFILE_FLAGS)

## Run clippy
.PHONY: clippy
clippy:
	cargo clippy $(CARGO_BUILD_FLAGS) $(CARGO_CROSS_FLAGS) $(CARGO_PROFILE_FLAGS)

## Run clippy for native architecture
.PHONY: clippy-native
clippy-native:
	cargo clippy $(CARGO_BUILD_FLAGS) $(CARGO_PROFILE_FLAGS)

## Run the project
.PHONY: run
run:
	cargo run -p kernel $(CARGO_BUILD_FLAGS) $(CARGO_CROSS_FLAGS) $(CARGO_PROFILE_FLAGS)

## Test the project
.PHONY: test
test:
	cargo nextest run
	cargo test --doc
