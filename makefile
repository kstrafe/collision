.PHONY:
all:
	cargo test -- --nocapture

.PHONY:
run:
	cargo build
	./target/debug/collision

.PHONY:
clip:
	cargo build --features dev

.PHONY:
rel:
	cargo build --release -j 12
	./target/release/collision

.PHONY:
fmt:
	cargo fmt -- --write-mode diff

.PHONY:
fmto:
	cargo fmt -- --write-mode overwrite
