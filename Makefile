SOCKET ?= $(HOME)/.grove/groved.sock

.PHONY: fmt clippy test ci tui groved tui-daemon root

fmt:
	cargo fmt --check

clippy:
	cargo clippy --all-targets --all-features -- -D warnings

test:
	cargo test

ci: fmt clippy test

tui:
	cargo run --bin grove -- tui

groved:
	cargo run --bin groved -- --socket "$(SOCKET)"

tui-daemon:
	cargo run --bin grove -- --socket "$(SOCKET)" tui

root:
	cargo run --bin grove --
