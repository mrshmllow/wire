# vim: set ft=make :

up:
	cd ./local-testing/ && nixos-shell -vv

build-dhat:
	cargo build --profile profiling --features dhat-heap
	@echo 'dhat binaries in target/profiling'
	@echo 'Example:'
	@echo 'WIRE_RUNTIME=/nix/store/...-runtime WIRE_KEY_AGENT=/nix/store/...-key_agent-0.1.0 PROJECT/target/profiling/wire apply ...'
