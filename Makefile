BIN := $(shell cargo metadata --no-deps --format-version 1 | jq -r ".packages[] | .targets[] | select(.kind | contains([\"bin\"])) | .name")
DBG_BIN := target/debug/$(BIN)
REL_BIN := target/release/$(BIN)
BENCH_BIN := target/release-full/$(BIN)
SOURCES := $(shell find src bin/src -name '*.rs') Cargo.toml Cargo.lock bin/Cargo.toml

FLAGS := --log-lvl debug
#BACKTRACE := RUST_BACKTRACE=1

CAP ?=
ifneq ($(CAP),)
FLAGS += --cap $(CAP)
endif

DEPTH ?=
ifneq ($(DEPTH),)
FLAGS += --depth $(DEPTH)
endif

.PHONY: default
default: debug release

.PHONY: run
run: debug
	$(BACKTRACE) $(DBG_BIN) $(FLAGS)

.PHONY: run-release
run-release: release
	$(BACKTRACE) $(REL_BIN) $(FLAGS)

.PHONY: bench
bench: $(BENCH_BIN)
	hyperfine --warmup 3 --runs 10 "$(BENCH_BIN) bench build-oct" --export-json bench.json

.PHONY: bench-manual
bench-manual: $(BENCH_BIN)
	$(BENCH_BIN) bench build-oct

$(BENCH_BIN): $(SOURCES)
	cargo build --profile release-full --features tracing/max_level_off

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
