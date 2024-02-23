#!/bin/bash
if [ "$1" == "devnet" ]; then
    solana config set --url https://api.devnet.solana.com
fi

if [ "$1" == "mainnet" ]; then
  solana config set --url https://solana-mainnet.g.alchemy.com/v2/a0Xic8r2YTu7uJ-O-Gn27SgmDTKaelhL
fi

cargo build-bpf --bpf-out-dir=./build
solana --keypair ./scripts/keys/deployer.json program deploy --program-id ./build/spl_staking-keypair.json ./build/spl_staking.so