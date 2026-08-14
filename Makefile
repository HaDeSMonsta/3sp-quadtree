BIN := $(shell cargo metadata --no-deps --format-version 1 | jq -r ".packages[0].name")
DBG_BIN := target/debug/$(BIN)
REL_BIN := target/release/$(BIN)
SOURCES := $(shell find src -name '*.rs') Cargo.toml Cargo.lock

.PHONY: default
default: debug release

.PHONY: run
run: debug
	$(DBG_BIN)

.PHONY: run-release
run-release: release
	$(REL_BIN)

.PHONY: check
check:
	cargo check

# Do not depend on this, it will always cause a rebuild, even without changes to sources
.PHONY: fmt
fmt:
	cargo fmt

.PHONY: debug
debug: $(DBG_BIN)

.PHONY: release
release: $(REL_BIN)

$(DBG_BIN): $(SOURCES)
	cargo build

$(REL_BIN): $(SOURCES)
	cargo build --release
