test:
	cargo test-sbf
deploy-local:
	solana config set --url http://127.0.0.1:8899
	cargo build-sbf --sbf-out-dir=./build
	solana-test-validator -r --mint 2yZgY7sdYK31n1rifYBXBd3hCWeS1CzqYwv3Mzty82vo --bpf-program 7iPzfTTkxYbEZy8JQLfsafApbzw5m9JYE7Amt7zeEDST ./build/spl_staking.so
resume-deploy:
	solana config set --url https://solana-mainnet.g.alchemy.com/v2/mSSt0hYTZTKiDCQwz8TLgt7duMQLueDZ
	solana program deploy --buffer scripts/keys/buffer.json --upgrade-authority scripts/keys/deployer.json --program-id ./build/spl_staking-keypair.json ./build/spl_staking.so