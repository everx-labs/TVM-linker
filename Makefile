UNAME = $(shell uname)
APP = tvm_linker

ifeq ($(UNAME), win)
EXT = .exe
endif
RELEASE = $(APP)$(EXT)
TARGET ?= $(RELEASE)

clean:
	rm -fr release
	cargo clean

test:
	cargo test
fmt:
	cargo fmt
fmt-check:
	cargo fmt --all -- --check
lint:
	cargo clippy --all-targets
qa: lint test

target/release/$(RELEASE):
	cargo build --release

release: target/release/$(RELEASE)
	mkdir -p release
	cp $< release/$(TARGET)
