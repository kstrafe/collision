all:
	cargo build
	./target/debug/collision

fmt:
	cargo fmt -- --write-mode diff
