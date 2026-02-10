
.PHONY: default
default:
	cargo build --release


.PHONY: fmt
fmt:
	cargo +nightly fmt
