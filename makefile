all:
	cargo build
	./target/debug/collision

rel:
	cargo build --release -j 12
	./target/release/collision

fmt:
	cargo fmt -- --write-mode diff

fmto:
	cargo fmt -- --write-mode overwrite
