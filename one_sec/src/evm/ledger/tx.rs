use ethnum::U256;
use ic_ethereum_types::Address;

use crate::{evm::writer::TxInput, flow::event::Input, numeric::Amount};

use super::Config;

/// A helper that builds a mint transaction for the given input.
pub fn build_mint_tx(input: &Input, config: &Config) -> TxInput {
    TxInput {
        contract: config.erc20_address,
        calldata: call_tx_with_input("mint", input),
        gas_limit: config.gas_limit_for_unlock_or_mint,
        cost_limit: config.max_tx_cost,
    }
}

/// A helper that builds an unlock transaction for the given input.
pub fn build_unlock_tx(input: &Input, config: &Config) -> TxInput {
    TxInput {
        contract: config.erc20_address,
        calldata: call_tx_with_input("transfer", input),
        gas_limit: config.gas_limit_for_unlock_or_mint,
        cost_limit: config.max_tx_cost,
    }
}

pub fn call_tx_with_input(name: &str, input: &Input) -> Vec<u8> {
    call_tx_with_address_and_amount(name, input.evm_account, input.evm_amount)
}

/// Return an RPL encoded `calldata` for calling a method with the given name
/// and passing the address of the recipient and the amount of tokens.
pub fn call_tx_with_address_and_amount(name: &str, to: Address, amount: Amount) -> Vec<u8> {
    #[allow(deprecated)]
    let function = ethabi::Function {
        name: name.into(),
        inputs: vec![
            ethabi::Param {
                name: "to".to_string(),
                kind: ethabi::ParamType::Address,
                internal_type: None,
            },
            ethabi::Param {
                name: "value".to_string(),
                kind: ethabi::ParamType::Uint(256),
                internal_type: None,
            },
        ],
        outputs: vec![],
        state_mutability: ethabi::StateMutability::NonPayable,
        constant: None,
    };

    function
        .encode_input(&[
            ethabi::Token::Address(to.into_bytes().into()),
            ethabi::Token::Uint(U256::new(amount.into_inner()).to_be_bytes().into()),
        ])
        .expect("BUG: failed to encode function input")
}

/// Return an RPL encoded `calldata` for calling a burn or lock method with the
/// given arguments. The actual name of the method depends on the length of data.
/// - data is 32 bytes: burn1/lock1 are called with (amount, data1)
/// - data is 64 bytes: burn2/lock2 are called with (amount, data1, data2)
pub fn call_burn_or_lock_tx(name: &str, amount: Amount, data: Vec<u8>) -> Vec<u8> {
    let mut input_abi = vec![];
    let mut input_value = vec![];

    input_abi.push(ethabi::Param {
        name: "amount".to_string(),
        kind: ethabi::ParamType::Uint(256),
        internal_type: None,
    });
    input_value.push(ethabi::Token::Uint(
        U256::new(amount.into_inner()).to_be_bytes().into(),
    ));

    let data_chunks = data.chunks(32);

    for (i, chunk) in data_chunks.enumerate() {
        input_abi.push(ethabi::Param {
            name: format!("data{}", i + 1),
            kind: ethabi::ParamType::FixedBytes(32),
            internal_type: None,
        });
        input_value.push(ethabi::Token::FixedBytes(chunk.to_vec()));
    }

    #[allow(deprecated)]
    let function = ethabi::Function {
        name: format!("{}{}", name, data.len() / 32),
        inputs: input_abi,
        outputs: vec![],
        state_mutability: ethabi::StateMutability::NonPayable,
        constant: None,
    };

    function
        .encode_input(&input_value)
        .expect("BUG: failed to encode function input")
}
