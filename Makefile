build:
	cargo build

test:
	cargo test -- --test-threads=1

run-local:
	cargo run

lint:
	cargo clippy
	cargo fmt

before-push: test lint
