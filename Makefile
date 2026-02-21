SOCKET ?= $(HOME)/.grove/groved.sock
REMOTE_USER ?= $(USER)
REMOTE_HOST ?=
REMOTE_SOCKET ?= /home/$(REMOTE_USER)/.grove/groved.sock
TUNNEL_KEY = $(subst /,-,$(subst :,-,$(subst @,-,$(REMOTE_USER)-$(REMOTE_HOST))))
LOCAL_SOCKET ?= $(HOME)/.grove/groved-$(TUNNEL_KEY).sock
SSH_CONTROL_PATH ?= $(HOME)/.grove/ssh-groved-$(TUNNEL_KEY).ctl

.PHONY: fmt clippy test ci tui debug-tui groved tui-daemon root tunnel-up tunnel-down tunnel-status

fmt:
	cargo fmt --check

clippy:
	cargo clippy --all-targets --all-features -- -D warnings

test:
	cargo test

ci: fmt clippy test

tui:
	cargo run --release --bin grove -- tui

debug-tui:
	RUST_BACKTRACE=1 cargo run --release --bin grove -- tui --debug-record

groved:
	cargo run --bin groved -- --socket "$(SOCKET)"

tui-daemon:
	cargo run --release --bin grove -- --socket "$(SOCKET)" tui

root:
	cargo run --bin grove --

tunnel-up:
	@test -n "$(REMOTE_HOST)" || (echo "REMOTE_HOST is required (example: make tunnel-up REMOTE_HOST=build.example.com)"; exit 2)
	@mkdir -p "$(HOME)/.grove"
	@ssh -fN \
		-M -S "$(SSH_CONTROL_PATH)" \
		-o ExitOnForwardFailure=yes \
		-o ServerAliveInterval=30 \
		-o ServerAliveCountMax=3 \
		-L "$(LOCAL_SOCKET):$(REMOTE_SOCKET)" \
		"$(REMOTE_USER)@$(REMOTE_HOST)"
	@echo "tunnel up: $(LOCAL_SOCKET) -> $(REMOTE_USER)@$(REMOTE_HOST):$(REMOTE_SOCKET)"

tunnel-down:
	@test -n "$(REMOTE_HOST)" || (echo "REMOTE_HOST is required (example: make tunnel-down REMOTE_HOST=build.example.com)"; exit 2)
	@ssh -S "$(SSH_CONTROL_PATH)" -O exit "$(REMOTE_USER)@$(REMOTE_HOST)" >/dev/null 2>&1 || true
	@rm -f "$(SSH_CONTROL_PATH)"
	@echo "tunnel down: $(REMOTE_USER)@$(REMOTE_HOST)"

tunnel-status:
	@test -n "$(REMOTE_HOST)" || (echo "REMOTE_HOST is required (example: make tunnel-status REMOTE_HOST=build.example.com)"; exit 2)
	@if ssh -S "$(SSH_CONTROL_PATH)" -O check "$(REMOTE_USER)@$(REMOTE_HOST)" >/dev/null 2>&1; then \
		echo "tunnel status: up ($(REMOTE_USER)@$(REMOTE_HOST), $(LOCAL_SOCKET))"; \
	else \
		echo "tunnel status: down ($(REMOTE_USER)@$(REMOTE_HOST))"; \
		exit 1; \
	fi
