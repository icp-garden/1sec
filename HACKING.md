# Hacking

This document explains how to set up a local EVM node and ICP canisters in order
to manually test the deposit ICP flow.

## Install Foundry

```shell
curl -L https://foundry.paradigm.xyz | bash
source $HOME/.bashrc
foundryup
```

Now you have `foundry`, `anvil`, `cast`.

## Start EVM node


Open a new terminal for the node and run for Base:
```shell
anvil --optimism
```

The `--optimism` mode is needed for Merkle proofs of the relayer.
If you're not running the relayer, then you can drop the flag.

Open a new terminal for the node and run for Arbitrum:
```shell
anvil --chain-id 31338 --port 8546
```

Open a new terminal for the node and run for Ethereum:
```shell
anvil --chain-id 31339 --port 8547
```

## Start dfx

Open a new terminal for dfx and run:
```shell
dfx start --clean
```

## Deploy contracts and canisters

Open a new terminal and source the script to get environment variables:
```shell
source ./local-deploy.sh
```

## Top up ICP balance

```shell
./icp_mint.sh <principal>
```

## Top up USDC balance

```shell
./usdc_mint.sh <address>
./usdc_mint.sh $EVM_ADDRESS
```

# Deposit USDC

```shell
./usdc_mint.sh <port> <address> <amount>
./usdc_deposit.sh $EVM_ADDRESS 1000000
```

# Withdraw USDC

```shell
./usdc_withdraw.sh <address> <amount>
./usdc_withdraw.sh $EVM_ADDRESS 1000000
```

## To interact with the frontend using an Evm Wallet. 

Go to the wallet extension and go to "add custom network".  

Select the correct rpc url: http://localhost:8545
Find the Chain ID associated to the rpc url: 
```shell
cast chain-id --rpc-url http://localhost:8545
```


# To check the ETH balance of the smart contract.
```shell
cast balance $CANISTER_EVM_ADDRESS
```

# To abi-encode args to verify contract
```shell
cast abi-encode "constructor(address,uint256)" $USDC_ERC20_ADDRESS 1000000
cast abi-encode "constructor(string,uint8,uint256)" "ICP" 8 100000000
0x000000000000000000000000000000000000000000000000000000000000006000000000000000000000000000000000000000000000000000000000000000080000000000000000000000000000000000000000000000000000000005f5e10000000000000000000000000000000000000000000000000000000000000000034943500000000000000000000000000000000000000000000000000000000000
cast abi-encode "constructor(string,uint8,uint256)" "ICP" 8 50000000
0x000000000000000000000000000000000000000000000000000000000000006000000000000000000000000000000000000000000000000000000000000000080000000000000000000000000000000000000000000000000000000002faf08000000000000000000000000000000000000000000000000000000000000000034943500000000000000000000000000000000000000000000000000000000000
```

# To verify a contract
```shell
forge verify-contract $ONESCEC_TOKEN_ADDRESS ./src/Token.sol:Token \
    --chain 1 \
    --watch \
    --rpc-url https://ethereum-rpc.publicnode.com \
    --verifier etherscan \
    --etherscan-api-key $ETHERSCAN_API_KEY  \
    --constructor-args 0x000000000000000000000000000000000000000000000000000000000000006000000000000000000000000000000000000000000000000000000000000000080000000000000000000000000000000000000000000000000000000005f5e10000000000000000000000000000000000000000000000000000000000000000034943500000000000000000000000000000000000000000000000000000000000
```

# To call a contract
```shell
cast send $ONESCEC_TOKEN_ADDRESS \
  "transfer(address,uint256)" \
  $DESTINATION_ADDRESS 1000000 \
  --private-key $EVM_PRIVATE_KEY \
  --rpc-url https://mainnet.base.org \
  --value 0.0005ether
```

# To deploy the ERC20 Locker
```shell
forge create --private-key $EVM_PRIVATE_KEY \
    --rpc-url https://mainnet.base.org  \
    --broadcast src/Locker.sol:Locker \
    --constructor-args 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913 1000000
forge create --private-key $EVM_PRIVATE_KEY \
    --rpc-url https://arbitrum.drpc.org  \
    --broadcast src/Locker.sol:Locker \
    --constructor-args 0xaf88d065e77c8cC2239327C5EDb3A432268e5831 1000000
forge create --private-key $EVM_PRIVATE_KEY \
    --rpc-url https://ethereum-rpc.publicnode.com  \
    --broadcast src/Locker.sol:Locker \
    --constructor-args 0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48 1000000
forge create --private-key $EVM_PRIVATE_KEY \
    --rpc-url https://ethereum-rpc.publicnode.com  \
    --broadcast src/Locker.sol:Locker \
    --constructor-args 0xdAC17F958D2ee523a2206206994597C13D831ec7 1000000
forge create --private-key $EVM_PRIVATE_KEY \
    --rpc-url https://ethereum-rpc.publicnode.com  \
    --broadcast src/Locker.sol:Locker \
    --constructor-args 0xcbB7C0000aB88B473b1f5aFd9ef808440eed33Bf 1000
forge create --private-key $EVM_PRIVATE_KEY \
    --rpc-url https://mainnet.base.org \
    --broadcast src/Locker.sol:Locker \
    --constructor-args 0xcbB7C0000aB88B473b1f5aFd9ef808440eed33Bf 1000
forge create --private-key $EVM_PRIVATE_KEY \
    --rpc-url https://arbitrum.drpc.org \
    --broadcast src/Locker.sol:Locker \
    --constructor-args 0xcbB7C0000aB88B473b1f5aFd9ef808440eed33Bf 1000
```

# To deploy the ERC20 Minter
```shell
forge create --private-key $EVM_PRIVATE_KEY \
    --rpc-url https://eth.blockrazor.xyz \
	--broadcast src/Token.sol:Token \
    --constructor-args "ICP" 8 100 000 000
forge create --private-key $EVM_PRIVATE_KEY \
    --rpc-url https://mainnet.base.org \
	--broadcast src/Token.sol:Token \
    --constructor-args "ICP" 8 100 000 000
forge create --private-key $EVM_PRIVATE_KEY \
    --rpc-url https://arbitrum.drpc.org \
	--broadcast src/Token.sol:Token \
    --constructor-args "ICP" 8 100 000 000

forge create --private-key $EVM_PRIVATE_KEY \
    --rpc-url https://eth.blockrazor.xyz \
	--broadcast src/Token.sol:Token \
    --constructor-args "ckBTC" 8 1000
forge create --private-key $EVM_PRIVATE_KEY \
    --rpc-url https://mainnet.base.org \
	--broadcast src/Token.sol:Token \
    --constructor-args "ckBTC" 8 1000
forge create --private-key $EVM_PRIVATE_KEY \
    --rpc-url https://arbitrum.drpc.org  \
	--broadcast src/Token.sol:Token \
    --constructor-args "ckBTC" 8 1000

forge create --private-key $EVM_PRIVATE_KEY \
    --rpc-url https://eth.blockrazor.xyz \
	--broadcast src/Token.sol:Token \
    --constructor-args "BOB" 8 10000000
forge create --private-key $EVM_PRIVATE_KEY \
    --rpc-url https://mainnet.base.org \
	--broadcast src/Token.sol:Token \
    --constructor-args "BOB" 8 10000000
forge create --private-key $EVM_PRIVATE_KEY \
    --rpc-url https://arbitrum.drpc.org \
	--broadcast src/Token.sol:Token \
    --constructor-args "BOB" 8 10000000

forge create --private-key $EVM_PRIVATE_KEY \
    --rpc-url https://eth.blockrazor.xyz \
	--broadcast src/Token.sol:Token \
    --constructor-args "GLDT" 8 100000000
forge create --private-key $EVM_PRIVATE_KEY \
    --rpc-url https://mainnet.base.org \
	--broadcast src/Token.sol:Token \
    --constructor-args "GLDT" 8 100000000
forge create --private-key $EVM_PRIVATE_KEY \
    --rpc-url https://arbitrum.drpc.org \
	--broadcast src/Token.sol:Token \
    --constructor-args "GLDT" 8 100000000
```

# Chain(ChainId): RPC
```shell
Ethereum(1): https://ethereum-rpc.publicnode.com
Base(8453): https://mainnet.base.org
Arbitrum(42161): https://arbitrum.drpc.org
```

# OneSec Contracts
- base usdc locker 0xAe2351B15cFf68b5863c6690dCA58Dce383bf45A
- arb usdc locker 0xAe2351B15cFf68b5863c6690dCA58Dce383bf45A
- eth usdc locker 0xAe2351B15cFf68b5863c6690dCA58Dce383bf45A
- icp on arb 0x00f3C42833C3170159af4E92dbb451Fb3F708917
- icp on base 0x00f3C42833C3170159af4E92dbb451Fb3F708917
- icp on eth 0x00f3C42833C3170159af4E92dbb451Fb3F708917
- eth usdt locker 0xc5AC945a0af0768929301A27D6f2a7770995fAeb - erc20 6 decimals 0xdAC17F958D2ee523a2206206994597C13D831ec7
- eth cbbtc locker 0xb6fa075AfaBC50e0D956c461DdfA37DCBD637C41 - erc20 8 decimals 0xcbB7C0000aB88B473b1f5aFd9ef808440eed33Bf
- base cbbtc locker 0xb6fa075AfaBC50e0D956c461DdfA37DCBD637C41 - erc20 8 decimals 0xcbB7C0000aB88B473b1f5aFd9ef808440eed33Bf
- arbitrum cbbtc locker 0xb6fa075AfaBC50e0D956c461DdfA37DCBD637C41 - erc20 8 decimals 0xcbB7C0000aB88B473b1f5aFd9ef808440eed33Bf

# OneSec Canister EVM Address
```
0x70AE25592209B57F62b3a3e832ab356228a2192C
```

# To deploy the USDC ledger
```shell
dfx canister install usdc_ledger --argument '(variant { Init = record { 
    minting_account = record { 
        owner = principal "5okwm-giaaa-aaaar-qbn6a-cai" 
    }; 
    fee_collector_account = opt record {
        owner = principal "54mbv-kyaaa-aaaar-qbn5a-cai";
    };
    transfer_fee = 10_000;
    token_symbol = "USDC"; 
    token_name = "USDC";
    feature_flags = opt record { icrc2 = true }; 
    max_memo_length = opt 8;
    decimals = opt 6;
    metadata = vec { 
        record { "icrc1:logo"; variant { Text = "data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iOTYiIGhlaWdodD0iOTYiIHZpZXdCb3g9IjAgMCA5NiA5NiIgZmlsbD0ibm9uZSIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj4KPHBhdGggZD0iTTQ4IDk1QzczLjk1NzQgOTUgOTUgNzMuOTU3NCA5NSA0OEM5NSAyMi4wNDI2IDczLjk1NzQgMSA0OCAxQzIyLjA0MjYgMSAxIDIyLjA0MjYgMSA0OEMxIDczLjk1NzQgMjIuMDQyNiA5NSA0OCA5NVoiIGZpbGw9IiMwQjUzQkYiLz4KPHBhdGggZD0iTTU2LjQ2MDkgMTMuNzc3OFYxOS44MjkxQzY4LjUzNDEgMjMuNDcxNiA3Ny4zNzU5IDM0LjY5MjggNzcuMzc1OSA0Ny45OTk3Qzc3LjM3NTkgNjEuMzA2NiA2OC41MzQxIDcyLjUyNzggNTYuNDYwOSA3Ni4xNzAzVjgyLjIyMTZDNzEuODUzNCA3OC40NjE2IDgzLjI1MDkgNjQuNTY3MiA4My4yNTA5IDQ3Ljk5OTdDODMuMjUwOSAzMS40MzIyIDcxLjg1MzQgMTcuNTM3OCA1Ni40NjA5IDEzLjc3NzhaIiBmaWxsPSJ3aGl0ZSIvPgo8cGF0aCBkPSJNMTguNjI1IDQ3Ljk5OTdDMTguNjI1IDM0LjY5MjggMjcuNDY2OSAyMy40NzE2IDM5LjU0IDE5LjgyOTFWMTMuNzc3OEMyNC4xNDc1IDE3LjUzNzggMTIuNzUgMzEuNDMyMiAxMi43NSA0Ny45OTk3QzEyLjc1IDY0LjU2NzIgMjQuMTQ3NSA3OC40NjE2IDM5LjU0IDgyLjIyMTZWNzYuMTcwM0MyNy40NjY5IDcyLjU1NzIgMTguNjI1IDYxLjMwNjYgMTguNjI1IDQ3Ljk5OTdaIiBmaWxsPSJ3aGl0ZSIvPgo8cGF0aCBkPSJNNjAuNjMxOSA1NC41NTA2QzYwLjYzMTkgNDIuNTM2MiA0MS44MDI1IDQ3LjQ3MTMgNDEuODAyNSA0MC44MzI1QzQxLjgwMjUgMzguNDUzMSA0My43MTE5IDM2LjkyNTYgNDcuMzU0NCAzNi45MjU2QzUxLjcwMTkgMzYuOTI1NiA1My4yIDM5LjA0MDYgNTMuNjcgNDEuODlINTkuNjYyNUM1OS4xMjc5IDM2LjU0MjYgNTYuMDU4OCAzMy4xNjYyIDUwLjkzODIgMzIuMTYwNFYyNy40Mzc1SDQ1LjA2MzJWMzEuOTkxOEMzOS40NTM0IDMyLjcwNjIgMzUuOTI3NSAzNS45NzMgMzUuOTI3NSA0MC44MzI1QzM1LjkyNzUgNTIuOTA1NiA1NC43ODYzIDQ4LjM4MTkgNTQuNzg2MyA1NC45MDMxQzU0Ljc4NjMgNTcuMzcwNiA1Mi40MDY5IDU5LjAxNTYgNDguMzgyNSA1OS4wMTU2QzQzLjEyNDQgNTkuMDE1NiA0MS4zOTEzIDU2LjY5NSA0MC43NDUgNTMuNDkzMUgzNC44OTk0QzM1LjI3ODEgNTkuMzUwMiAzOC44ODk3IDYzLjAxNTkgNDUuMDYzMiA2My45MzA3VjY4LjU2MjVINTAuOTM4MlY2My45OTIzQzU2Ljk2MzMgNjMuMjEzOSA2MC42MzE5IDU5LjcwODkgNjAuNjMxOSA1NC41NTA2WiIgZmlsbD0id2hpdGUiLz4KPC9zdmc+Cg==" }}
    };
    initial_balances = vec {}; 
    archive_options = record { 
        num_blocks_to_archive = 1000;
        trigger_threshold = 2000;
        max_message_size_bytes = null; 
        cycles_for_archive_creation = opt 10_000_000_000_000; 
        node_max_memory_size_bytes = opt 3_221_225_472; 
        controller_id = principal "r7inp-6aaaa-aaaaa-aaabq-cai" 
    } 
}})' --wallet 54mbv-kyaaa-aaaar-qbn5a-cai --ic


dfx canister install usdc_ledger --argument '(variant { Upgrade = opt record { 
    metadata = opt vec { 
        record { "icrc1:logo"; variant { Text = "data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iOTYiIGhlaWdodD0iOTYiIHZpZXdCb3g9IjAgMCA5NiA5NiIgZmlsbD0ibm9uZSIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj4KPHBhdGggZD0iTTQ4IDk1QzczLjk1NzQgOTUgOTUgNzMuOTU3NCA5NSA0OEM5NSAyMi4wNDI2IDczLjk1NzQgMSA0OCAxQzIyLjA0MjYgMSAxIDIyLjA0MjYgMSA0OEMxIDczLjk1NzQgMjIuMDQyNiA5NSA0OCA5NVoiIGZpbGw9IiMwQjUzQkYiLz4KPHBhdGggZD0iTTU2LjQ2MDkgMTMuNzc3OFYxOS44MjkxQzY4LjUzNDEgMjMuNDcxNiA3Ny4zNzU5IDM0LjY5MjggNzcuMzc1OSA0Ny45OTk3Qzc3LjM3NTkgNjEuMzA2NiA2OC41MzQxIDcyLjUyNzggNTYuNDYwOSA3Ni4xNzAzVjgyLjIyMTZDNzEuODUzNCA3OC40NjE2IDgzLjI1MDkgNjQuNTY3MiA4My4yNTA5IDQ3Ljk5OTdDODMuMjUwOSAzMS40MzIyIDcxLjg1MzQgMTcuNTM3OCA1Ni40NjA5IDEzLjc3NzhaIiBmaWxsPSJ3aGl0ZSIvPgo8cGF0aCBkPSJNMTguNjI1IDQ3Ljk5OTdDMTguNjI1IDM0LjY5MjggMjcuNDY2OSAyMy40NzE2IDM5LjU0IDE5LjgyOTFWMTMuNzc3OEMyNC4xNDc1IDE3LjUzNzggMTIuNzUgMzEuNDMyMiAxMi43NSA0Ny45OTk3QzEyLjc1IDY0LjU2NzIgMjQuMTQ3NSA3OC40NjE2IDM5LjU0IDgyLjIyMTZWNzYuMTcwM0MyNy40NjY5IDcyLjU1NzIgMTguNjI1IDYxLjMwNjYgMTguNjI1IDQ3Ljk5OTdaIiBmaWxsPSJ3aGl0ZSIvPgo8cGF0aCBkPSJNNjAuNjMxOSA1NC41NTA2QzYwLjYzMTkgNDIuNTM2MiA0MS44MDI1IDQ3LjQ3MTMgNDEuODAyNSA0MC44MzI1QzQxLjgwMjUgMzguNDUzMSA0My43MTE5IDM2LjkyNTYgNDcuMzU0NCAzNi45MjU2QzUxLjcwMTkgMzYuOTI1NiA1My4yIDM5LjA0MDYgNTMuNjcgNDEuODlINTkuNjYyNUM1OS4xMjc5IDM2LjU0MjYgNTYuMDU4OCAzMy4xNjYyIDUwLjkzODIgMzIuMTYwNFYyNy40Mzc1SDQ1LjA2MzJWMzEuOTkxOEMzOS40NTM0IDMyLjcwNjIgMzUuOTI3NSAzNS45NzMgMzUuOTI3NSA0MC44MzI1QzM1LjkyNzUgNTIuOTA1NiA1NC43ODYzIDQ4LjM4MTkgNTQuNzg2MyA1NC45MDMxQzU0Ljc4NjMgNTcuMzcwNiA1Mi40MDY5IDU5LjAxNTYgNDguMzgyNSA1OS4wMTU2QzQzLjEyNDQgNTkuMDE1NiA0MS4zOTEzIDU2LjY5NSA0MC43NDUgNTMuNDkzMUgzNC44OTk0QzM1LjI3ODEgNTkuMzUwMiAzOC44ODk3IDYzLjAxNTkgNDUuMDYzMiA2My45MzA3VjY4LjU2MjVINTAuOTM4MlY2My45OTIzQzU2Ljk2MzMgNjMuMjEzOSA2MC42MzE5IDU5LjcwODkgNjAuNjMxOSA1NC41NTA2WiIgZmlsbD0id2hpdGUiLz4KPC9zdmc+Cg==" }}
    };
}})' --wallet 54mbv-kyaaa-aaaar-qbn5a-cai --ic --mode upgrade --upgrade-unchanged

dfx canister install usdt_ledger --argument '(variant { Upgrade = opt record { 
    index_principal = opt principal "fvhuy-zyaaa-aaaar-qbpha-cai";
}})' --wallet 54mbv-kyaaa-aaaar-qbn5a-cai --ic --mode upgrade --upgrade-unchanged

dfx canister install cbbtc_ledger --argument '(variant { Upgrade = opt record { 
    index_principal = opt principal "fsgsm-uaaaa-aaaar-qbphq-cai";
}})' --wallet 54mbv-kyaaa-aaaar-qbn5a-cai --ic --mode upgrade --upgrade-unchanged
```

# To deploy index
```shell
dfx canister create --ic usdc_index --subnet-type fiduciary --controller 54mbv-kyaaa-aaaar-qbn5a-cai --wallet 54mbv-kyaaa-aaaar-qbn5a-cai
dfx canister install usdc_index --argument '(opt variant { Init = record { 
    ledger_id = principal "53nhb-haaaa-aaaar-qbn5q-cai";
    retrieve_blocks_from_ledger_interval_seconds = opt 30;
}})' --wallet 54mbv-kyaaa-aaaar-qbn5a-cai --ic
dfx canister install usdt_index --argument '(opt variant { Init = record { 
    ledger_id = principal "ij33n-oiaaa-aaaar-qbooa-cai";
    retrieve_blocks_from_ledger_interval_seconds = opt 30;
}})' --wallet 54mbv-kyaaa-aaaar-qbn5a-cai --ic
dfx canister install cbbtc_index --argument '(opt variant { Init = record { 
    ledger_id = principal "io25z-dqaaa-aaaar-qbooq-cai";
    retrieve_blocks_from_ledger_interval_seconds = opt 30;
}})' --wallet 54mbv-kyaaa-aaaar-qbn5a-cai --ic
```

# To deploy the USDT ledger
```shell
dfx canister install usdt_ledger --argument '(variant { Init = record { 
    minting_account = record { 
        owner = principal "5okwm-giaaa-aaaar-qbn6a-cai" 
    }; 
    fee_collector_account = opt record {
        owner = principal "54mbv-kyaaa-aaaar-qbn5a-cai";
    };
    transfer_fee = 10_000;
    token_symbol = "USDT"; 
    token_name = "USDT";
    feature_flags = opt record { icrc2 = true }; 
    max_memo_length = opt 8;
    decimals = opt 6;
    metadata = vec { 
        record { "icrc1:logo"; variant { Text = "data:image/svg+xml;base64,PHN2ZyBpZD0iTGF5ZXJfMSIgZGF0YS1uYW1lPSJMYXllciAxIiB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCAzMzkuNDMgMjk1LjI3Ij48dGl0bGU+dGV0aGVyLXVzZHQtbG9nbzwvdGl0bGU+PHBhdGggZD0iTTYyLjE1LDEuNDVsLTYxLjg5LDEzMGEyLjUyLDIuNTIsMCwwLDAsLjU0LDIuOTRMMTY3Ljk1LDI5NC41NmEyLjU1LDIuNTUsMCwwLDAsMy41MywwTDMzOC42MywxMzQuNGEyLjUyLDIuNTIsMCwwLDAsLjU0LTIuOTRsLTYxLjg5LTEzMEEyLjUsMi41LDAsMCwwLDI3NSwwSDY0LjQ1YTIuNSwyLjUsMCwwLDAtMi4zLDEuNDVoMFoiIHN0eWxlPSJmaWxsOiM1MGFmOTU7ZmlsbC1ydWxlOmV2ZW5vZGQiLz48cGF0aCBkPSJNMTkxLjE5LDE0NC44djBjLTEuMi4wOS03LjQsMC40Ni0yMS4yMywwLjQ2LTExLDAtMTguODEtLjMzLTIxLjU1LTAuNDZ2MGMtNDIuNTEtMS44Ny03NC4yNC05LjI3LTc0LjI0LTE4LjEzczMxLjczLTE2LjI1LDc0LjI0LTE4LjE1djI4LjkxYzIuNzgsMC4yLDEwLjc0LjY3LDIxLjc0LDAuNjcsMTMuMiwwLDE5LjgxLS41NSwyMS0wLjY2di0yOC45YzQyLjQyLDEuODksNzQuMDgsOS4yOSw3NC4wOCwxOC4xM3MtMzEuNjUsMTYuMjQtNzQuMDgsMTguMTJoMFptMC0zOS4yNVY3OS42OGg1OS4yVjQwLjIzSDg5LjIxVjc5LjY4SDE0OC40djI1Ljg2Yy00OC4xMSwyLjIxLTg0LjI5LDExLjc0LTg0LjI5LDIzLjE2czM2LjE4LDIwLjk0LDg0LjI5LDIzLjE2djgyLjloNDIuNzhWMTUxLjgzYzQ4LTIuMjEsODQuMTItMTEuNzMsODQuMTItMjMuMTRzLTM2LjA5LTIwLjkzLTg0LjEyLTIzLjE1aDBabTAsMGgwWiIgc3R5bGU9ImZpbGw6I2ZmZjtmaWxsLXJ1bGU6ZXZlbm9kZCIvPjwvc3ZnPg==" }}
    };
    initial_balances = vec {}; 
    archive_options = record { 
        num_blocks_to_archive = 1000;
        trigger_threshold = 2000;
        max_message_size_bytes = null; 
        cycles_for_archive_creation = opt 10_000_000_000_000; 
        node_max_memory_size_bytes = opt 3_221_225_472; 
        controller_id = principal "r7inp-6aaaa-aaaaa-aaabq-cai" 
    } 
}})' --wallet 54mbv-kyaaa-aaaar-qbn5a-cai --ic 
```

# To deploy the cbBTC ledger
```shell
dfx canister install cbbtc_ledger --argument '(variant { Init = record { 
    minting_account = record { 
        owner = principal "5okwm-giaaa-aaaar-qbn6a-cai" 
    }; 
    fee_collector_account = opt record {
        owner = principal "54mbv-kyaaa-aaaar-qbn5a-cai";
    };
    transfer_fee = 20;
    token_symbol = "cbBTC"; 
    token_name = "cbBTC";
    feature_flags = opt record { icrc2 = true }; 
    max_memo_length = opt 32;
    decimals = opt 8;
    metadata = vec { 
        record { "icrc1:logo"; variant { Text = "data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHdpZHRoPSI1MDAiIGhlaWdodD0iNTAwIiB2aWV3Qm94PSIwIDAgNTAwIDUwMCIgZmlsbD0ibm9uZSI+CjxnIGNsaXAtcGF0aD0idXJsKCNjbGlwMF80MTYwXzEyNzQpIj4KPHBhdGggZD0iTTI1MCA1MDBDMzg4LjA3MSA1MDAgNTAwIDM4OC4wNzEgNTAwIDI1MEM1MDAgMTExLjkyOSAzODguMDcxIDAgMjUwIDBDMTExLjkyOSAwIDAgMTExLjkyOSAwIDI1MEMwIDM4OC4wNzEgMTExLjkyOSA1MDAgMjUwIDUwMFoiIGZpbGw9IndoaXRlIi8+CjxwYXRoIGZpbGwtcnVsZT0iZXZlbm9kZCIgY2xpcC1ydWxlPSJldmVub2RkIiBkPSJNMjUwIDQ3NkMzNzQuODE2IDQ3NiA0NzYgMzc0LjgxNiA0NzYgMjUwQzQ3NiAxMjUuMTg0IDM3NC44MTYgMjQgMjUwIDI0QzEyNS4xODQgMjQgMjQgMTI1LjE4NCAyNCAyNTBDMjQgMzc0LjgxNiAxMjUuMTg0IDQ3NiAyNTAgNDc2Wk01MDAgMjUwQzUwMCAzODguMDcxIDM4OC4wNzEgNTAwIDI1MCA1MDBDMTExLjkyOSA1MDAgMCAzODguMDcxIDAgMjUwQzAgMTExLjkyOSAxMTEuOTI5IDAgMjUwIDBDMzg4LjA3MSAwIDUwMCAxMTEuOTI5IDUwMCAyNTBaIiBmaWxsPSIjMDA1MkZGIi8+CjxwYXRoIGQ9Ik0zMzYuMDMyIDIyMy4zOEMzNDAuNTIzIDE5Ni41MDcgMzIwLjI4NiAxODEuNzI1IDI5Mi40NTggMTcxLjM3NUwzMDIuMTI1IDEzNS4yOThMMjgwLjA2NSAxMjkuMzg3TDI3MC41ODMgMTY0Ljc3NEMyNjQuNzgxIDE2My4yMiAyNTguODY0IDE2MS42MzQgMjUyLjg4NSAxNjAuMjc5TDI2Mi40MTMgMTI0LjcxOEwyNDAuNDY4IDExOC44MzhMMjMwLjc0IDE1NS4xNDVDMjI1Ljk3MiAxNTMuODY4IDIyMS4yNjEgMTUyLjYwNSAyMTYuNjY1IDE1MS4zNzRMMTg1Ljk4OCAxNDMuMTU0TDE3OS42NjIgMTY2Ljc2NUMxNzkuNjYyIDE2Ni43NjUgMTk2LjA1NCAxNzAuODQ5IDE5NS42ODkgMTcxLjA2QzIwMS45NjQgMTcyLjAwMiAyMDYuMzgyIDE3Ny44MDQgMjA1LjYxMiAxODQuMTI0TDE3OC45ODIgMjgzLjUwOUMxNzguMzEzIDI4NS41NDYgMTc2LjgxMyAyODcuMjM3IDE3NC44OTIgMjg4LjIwMUMxNzIuOTcyIDI4OS4xNjQgMTcwLjcxOSAyODkuMjk5IDE2OC42OTcgMjg4LjU3M0MxNjguOTIyIDI4OC44NzkgMTUyLjY2OSAyODQuMjc4IDE1Mi42NjkgMjg0LjI3OEwxNDEuMjM3IDMwOS40NzZMMTg1LjgxNyAzMjEuNDIxTDE3NS45MzQgMzU4LjMwM0wxOTcuOTk0IDM2NC4yMTRMMjA3LjcyMyAzMjcuOTA3QzIxMy41ODMgMzI5LjQ3NyAyMTkuNSAzMzEuMDYyIDIyNS4zNTkgMzMyLjYzM0wyMTUuNjc3IDM2OC43NjdMMjM3Ljc5NCAzNzQuNjk0TDI0Ny41NjkgMzM4LjIxNEMyODUuNDUzIDM0NS45NjQgMzEzLjk0IDM0My43NDUgMzI2LjUxNiAzMDkuNjc5QzMzNi42ODUgMjgyLjI5NSAzMjYuODI4IDI2Ni4yMzEgMzA3LjA0MiAyNTUuNTExQzMyMS42ODEgMjUyLjM1MyAzMzIuODU5IDI0My4wMzQgMzM2LjAzMiAyMjMuMzhWMjIzLjM4Wk0yODQuMDk4IDI5My41NzJDMjc2Ljc1NiAzMjAuOTc1IDIzMC41OTUgMzA1LjI4MSAyMTUuNjU4IDMwMS4yNzlMMjI4LjY5NiAyNTIuNjIxQzI0My42MTcgMjU2LjY4IDI5MS43NjQgMjY0Ljk2MyAyODQuMDk4IDI5My41NzJWMjkzLjU3MlpNMjkyLjI1MiAyMjIuMjRDMjg1LjUyNSAyNDcuMzQ0IDI0Ny4xMTkgMjMzLjcyOSAyMzQuNzExIDIzMC40MDRMMjQ2LjU0OCAxODYuMjI3QzI1OC45NDEgMTg5LjYwOSAyOTkuMjQgMTk2LjE1OCAyOTIuMjUyIDIyMi4yNFoiIGZpbGw9IiMwMDUyRkYiLz4KPC9nPgo8ZGVmcz4KPGNsaXBQYXRoIGlkPSJjbGlwMF80MTYwXzEyNzQiPgo8cmVjdCB3aWR0aD0iNTAwIiBoZWlnaHQ9IjUwMCIgZmlsbD0id2hpdGUiLz4KPC9jbGlwUGF0aD4KPC9kZWZzPgo8L3N2Zz4=" }}
    };
    initial_balances = vec {}; 
    archive_options = record { 
        num_blocks_to_archive = 1000;
        trigger_threshold = 2000;
        max_message_size_bytes = null; 
        cycles_for_archive_creation = opt 10_000_000_000_000; 
        node_max_memory_size_bytes = opt 3_221_225_472; 
        controller_id = principal "r7inp-6aaaa-aaaaa-aaabq-cai" 
    } 
}})' --wallet 54mbv-kyaaa-aaaar-qbn5a-cai --ic 
```

# Backup and restore
Backup:
```shell
cd backup
cargo run
```
The backup tool run in a loop and fetched new events. You need to `Ctrl+C` to exit.

Restore:
```shell
# Only do this for throw-away identities that don't hold any assets!
dfx identity use default
dfx identity export default > backup/local.pem
dfx start --clean # in another tab or use --background

dfx canister create one_sec_staging
cargo build --target wasm32-unknown-unknown --release -p one_sec --locked --features=dev
dfx canister install one_sec_staging --mode=reinstall --wasm ./target/wasm32-unknown-unknown/release/one_sec.wasm --argument-file ./one_sec/prod_init_arg.did

cd backup
cargo run -- --canister-id zvjow-lyaaa-aaaar-qap7q-cai --icp-url http://127.0.0.1:8080 --restore --identity ./local.pem
cd ..

dfx deploy one_sec_staging --argument-file one_sec/prod_upgrade_arg.did
dfx canister call one_sec_staging pause_all_tasks '()'

# Try another upgrade to see that there are no pre-upgrade failures.
dfx deploy one_sec_staging --argument-file one_sec/prod_upgrade_arg.did
dfx canister call one_sec_staging pause_all_tasks '()'
```

# Monitoring the cycles of a canister 
```shell
dfx canister call w4cjz-riaaa-aaaam-acuza-cai monitor_canister '(principal "io25z-dqaaa-aaaar-qbooq-cai", variant {Metrics})' --ic
```

# Deploy a relayer

Generate an identity:
```shell
openssl ecparam -name secp256k1 -genkey | openssl ec -aes256 -out encrypted_ec_key.pem
```

Then run:
```shell
cargo run -- --base-url http://localhost:8545 --arbitrum-url http://localhost:8546 --ethereum-url http://localhost:8547 --identity ./encrypted_ec_key.pem --icp-url http://localhost:8080 --forward-chains base
```

Then you should have something like this:
```shell
Relayer ICP principal: [Principal]
Relayer EVM address: [Address]
```
Send ETH to the evm address on every chain. 
Add the principal to the config. 