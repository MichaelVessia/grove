.PHONY: fmt check clippy test precommit ci tui debug-tui root

fmt:
	cargo fmt --check

check:
	cargo check --all-targets --all-features

clippy:
	cargo clippy --all-targets --all-features -- -D warnings

test:
	cargo test

precommit: fmt check

ci: fmt clippy test

tui:
	cargo run --release --bin grove -- tui

debug-tui:
	RUST_BACKTRACE=1 cargo run --release --bin grove -- tui --debug-record

root:
	cargo run --bin grove --
