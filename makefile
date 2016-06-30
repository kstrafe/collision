all:
	cargo build
	./target/debug/collision

fmt:
	cargo fmt -- --write-mode diff

fmto:
	cargo fmt -- --write-mode overwrite
