.PHONY: goldens ci

goldens:
	UMORIA_REGEN_GOLDEN=1 tools/capture/regen.sh

ci:
	cargo fmt --check
	cargo clippy --all-targets -- -D warnings
	cargo build
	cargo test
