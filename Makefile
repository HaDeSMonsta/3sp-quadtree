BIN := $(shell cargo metadata --no-deps --format-version 1 | jq -r ".packages[0].name")
DBG_BIN := target/debug/$(BIN)
REL_BIN := target/release/$(BIN)
SOURCES := $(shell find src -name '*.rs') Cargo.toml Cargo.lock

CAP ?=
CAP_FLAG := $(if $(CAP),--cap $(CAP))

DEPTH ?=
DEPTH_FLAG := $(if $(DEPTH),--depth $(DEPTH))

.PHONY: default
default: debug release

.PHONY: run
run: debug
	$(DBG_BIN) $(CAP_FLAG) $(DEPTH_FLAG)

.PHONY: run-release
run-release: release
	$(REL_BIN) $(CAP_FLAG) $(DEPTH_FLAG)

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
