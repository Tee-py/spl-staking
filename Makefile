test:
	cargo test-sbf
deploy-local:
	solana config set --url http://127.0.0.1:8899
	cargo build-sbf --sbf-out-dir=./build
	solana-test-validator -r --mint E2F3fsS1HpsLb2VpEgsA5ztfo83CWFWW4jWpC6FvJ6qR --bpf-program 4yBTZXsuz7c1X3PJF4PPCJr8G6HnNAgRvzAWVoFZMncH ./build/spl_staking.so