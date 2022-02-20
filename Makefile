build:
	cargo build

test:
	cargo test -- --test-threads=1

run-local:
	cargo run

lint:
	cargo clippy  --all-features --tests
	cargo fmt

before-push: test lint
