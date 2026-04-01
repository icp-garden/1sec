use alloy::{
    network::{AnyTxEnvelope, UnknownTxEnvelope},
    primitives::{Address, Bytes, FixedBytes, U256, U64},
};
use alloy_consensus::{transaction::Recovered, EthereumTxEnvelope, TxEip4844Variant};
use alloy_eips::{Encodable2718, Typed2718};
use alloy_rlp::{Encodable, Header};
use eyre::eyre;
use serde::Deserialize;

const ARBITRUM_DEPOSIT_TX_TYPE: u8 = 0x64;
const ARBITRUM_UNSIGNED_TX_TYPE: u8 = 0x65;
const ARBITRUM_CONTRACT_TX_TYPE: u8 = 0x66;
const ARBITRUM_RETRY_TX_TYPE: u8 = 0x68;
const ARBITRUM_SUBMIT_RETRYABLE_TX_TYPE: u8 = 0x69;
const ARBITRUM_INTERNAL_TX_TYPE: u8 = 0x6A;

struct UnknownTxEnvelopeWithSigner {
    signer: Address,
    inner: UnknownTxEnvelope,
}

#[derive(Debug)]
pub enum ArbitrumTxEnvelope {
    Ethereum(EthereumTxEnvelope<TxEip4844Variant>),
    Arbitrum(ArbitrumTxVariant),
}

impl TryFrom<Recovered<AnyTxEnvelope>> for ArbitrumTxEnvelope {
    type Error = eyre::Error;

    fn try_from(value: Recovered<AnyTxEnvelope>) -> Result<Self, Self::Error> {
        match value.inner().clone() {
            AnyTxEnvelope::Ethereum(inner) => Ok(ArbitrumTxEnvelope::Ethereum(inner)),
            AnyTxEnvelope::Unknown(inner) => Ok(ArbitrumTxEnvelope::Arbitrum(
                ArbitrumTxVariant::try_from(UnknownTxEnvelopeWithSigner {
                    signer: value.signer(),
                    inner,
                })?,
            )),
        }
    }
}

impl Typed2718 for ArbitrumTxEnvelope {
    fn ty(&self) -> u8 {
        match self {
            ArbitrumTxEnvelope::Ethereum(inner) => inner.ty(),
            ArbitrumTxEnvelope::Arbitrum(inner) => inner.ty(),
        }
    }
}

impl Encodable2718 for ArbitrumTxEnvelope {
    fn encode_2718_len(&self) -> usize {
        match self {
            ArbitrumTxEnvelope::Ethereum(inner) => inner.encode_2718_len(),
            ArbitrumTxEnvelope::Arbitrum(inner) => inner.encode_2718_len(),
        }
    }

    fn encode_2718(&self, out: &mut dyn alloy_rlp::BufMut) {
        match self {
            ArbitrumTxEnvelope::Ethereum(inner) => inner.encode_2718(out),
            ArbitrumTxEnvelope::Arbitrum(inner) => inner.encode_2718(out),
        }
    }
}

#[derive(Debug)]
pub enum ArbitrumTxVariant {
    Deposit(ArbitrumDepositTx),
    Unsigned(ArbitrumUnsignedTx),
    Contract(ArbitrumContractTx),
    Retry(ArbitrumRetryTx),
    SubmitRetryable(ArbitrumSubmitRetryableTx),
    Internal(ArbitrumInternalTx),
}

impl ArbitrumTxVariant {
    fn rlp_header(&self) -> Header {
        Header {
            list: true,
            payload_length: self.rlp_encoded_fields_length(),
        }
    }

    fn rlp_encoded_fields_length(&self) -> usize {
        match self {
            ArbitrumTxVariant::Deposit(inner) => inner.rlp_encoded_fields_length(),
            ArbitrumTxVariant::Unsigned(inner) => inner.rlp_encoded_fields_length(),
            ArbitrumTxVariant::Contract(inner) => inner.rlp_encoded_fields_length(),
            ArbitrumTxVariant::Retry(inner) => inner.rlp_encoded_fields_length(),
            ArbitrumTxVariant::SubmitRetryable(inner) => inner.rlp_encoded_fields_length(),
            ArbitrumTxVariant::Internal(inner) => inner.rlp_encoded_fields_length(),
        }
    }

    fn rlp_encode_fields(&self, out: &mut dyn alloy_rlp::BufMut) {
        match self {
            ArbitrumTxVariant::Deposit(inner) => inner.rlp_encode_fields(out),
            ArbitrumTxVariant::Unsigned(inner) => inner.rlp_encode_fields(out),
            ArbitrumTxVariant::Contract(inner) => inner.rlp_encode_fields(out),
            ArbitrumTxVariant::Retry(inner) => inner.rlp_encode_fields(out),
            ArbitrumTxVariant::SubmitRetryable(inner) => inner.rlp_encode_fields(out),
            ArbitrumTxVariant::Internal(inner) => inner.rlp_encode_fields(out),
        }
    }
}

impl Typed2718 for ArbitrumTxVariant {
    fn ty(&self) -> u8 {
        match self {
            ArbitrumTxVariant::Deposit(_) => ARBITRUM_DEPOSIT_TX_TYPE,
            ArbitrumTxVariant::Unsigned(_) => ARBITRUM_UNSIGNED_TX_TYPE,
            ArbitrumTxVariant::Contract(_) => ARBITRUM_CONTRACT_TX_TYPE,
            ArbitrumTxVariant::Retry(_) => ARBITRUM_RETRY_TX_TYPE,
            ArbitrumTxVariant::SubmitRetryable(_) => ARBITRUM_SUBMIT_RETRYABLE_TX_TYPE,
            ArbitrumTxVariant::Internal(_) => ARBITRUM_INTERNAL_TX_TYPE,
        }
    }
}

impl Encodable2718 for ArbitrumTxVariant {
    fn encode_2718_len(&self) -> usize {
        self.ty().length() + self.rlp_header().length() + self.rlp_encoded_fields_length()
    }

    fn encode_2718(&self, out: &mut dyn alloy_rlp::BufMut) {
        out.put_u8(self.ty());
        self.rlp_header().encode(out);
        self.rlp_encode_fields(out);
    }
}

impl TryFrom<UnknownTxEnvelopeWithSigner> for ArbitrumTxVariant {
    type Error = eyre::Error;

    fn try_from(value: UnknownTxEnvelopeWithSigner) -> Result<Self, Self::Error> {
        let ty = value.inner.ty();
        if ty == ARBITRUM_DEPOSIT_TX_TYPE {
            Ok(ArbitrumTxVariant::Deposit(ArbitrumDepositTx::try_from(
                value,
            )?))
        } else if ty == ARBITRUM_UNSIGNED_TX_TYPE {
            Ok(ArbitrumTxVariant::Unsigned(ArbitrumUnsignedTx::try_from(
                value,
            )?))
        } else if ty == ARBITRUM_CONTRACT_TX_TYPE {
            Ok(ArbitrumTxVariant::Contract(ArbitrumContractTx::try_from(
                value,
            )?))
        } else if ty == ARBITRUM_RETRY_TX_TYPE {
            Ok(ArbitrumTxVariant::Retry(ArbitrumRetryTx::try_from(value)?))
        } else if ty == ARBITRUM_SUBMIT_RETRYABLE_TX_TYPE {
            Ok(ArbitrumTxVariant::SubmitRetryable(
                ArbitrumSubmitRetryableTx::try_from(value)?,
            ))
        } else if ty == ARBITRUM_INTERNAL_TX_TYPE {
            Ok(ArbitrumTxVariant::Internal(ArbitrumInternalTx::try_from(
                value,
            )?))
        } else {
            Err(eyre!(
                "Unknown transaction type {} in tx {}",
                value.inner.ty(),
                value.inner.hash
            ))
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ArbitrumDepositTx {
    #[serde(rename = "chainId")]
    pub chain_id: U64,
    #[serde(rename = "requestId")]
    pub l1_request_id: FixedBytes<32>,
    #[serde(skip_deserializing)]
    pub from: Address,
    pub to: Address,
    pub value: U256,
}

impl ArbitrumDepositTx {
    fn rlp_encoded_fields_length(&self) -> usize {
        self.chain_id.length()
            + self.l1_request_id.length()
            + self.from.length()
            + self.to.length()
            + self.value.length()
    }

    fn rlp_encode_fields(&self, out: &mut dyn alloy_rlp::BufMut) {
        self.chain_id.encode(out);
        self.l1_request_id.encode(out);
        self.from.encode(out);
        self.to.encode(out);
        self.value.encode(out);
    }
}

impl TryFrom<UnknownTxEnvelopeWithSigner> for ArbitrumDepositTx {
    type Error = eyre::Error;

    fn try_from(value: UnknownTxEnvelopeWithSigner) -> Result<Self, Self::Error> {
        let mut result: ArbitrumDepositTx = parse_tx_fields(value.inner)?;
        result.from = value.signer;
        Ok(result)
    }
}

#[derive(Debug, Deserialize)]
pub struct ArbitrumUnsignedTx {
    #[serde(rename = "chainId")]
    pub chain_id: U64,
    #[serde(skip_deserializing)]
    pub from: Address,
    pub nonce: u64,
    #[serde(rename = "maxFeePerGas")]
    pub gas_fee_cap: U256,
    pub gas: U64,
    pub to: Address,
    pub value: U256,
    #[serde(rename = "input")]
    pub data: Bytes,
}

impl ArbitrumUnsignedTx {
    fn rlp_encoded_fields_length(&self) -> usize {
        self.chain_id.length()
            + self.from.length()
            + self.nonce.length()
            + self.gas_fee_cap.length()
            + self.gas.length()
            + self.to.length()
            + self.value.length()
            + self.data.length()
    }

    fn rlp_encode_fields(&self, out: &mut dyn alloy_rlp::BufMut) {
        self.chain_id.encode(out);
        self.from.encode(out);
        self.nonce.encode(out);
        self.gas_fee_cap.encode(out);
        self.gas.encode(out);
        self.to.encode(out);
        self.value.encode(out);
        self.data.encode(out);
    }
}

impl TryFrom<UnknownTxEnvelopeWithSigner> for ArbitrumUnsignedTx {
    type Error = eyre::Error;

    fn try_from(value: UnknownTxEnvelopeWithSigner) -> Result<Self, Self::Error> {
        let mut result: ArbitrumUnsignedTx = parse_tx_fields(value.inner)?;
        result.from = value.signer;
        Ok(result)
    }
}

#[derive(Debug, Deserialize)]
pub struct ArbitrumContractTx {
    #[serde(rename = "chainId")]
    pub chain_id: U64,
    #[serde(rename = "requestId")]
    pub request_id: FixedBytes<32>,
    #[serde(skip_deserializing)]
    pub from: Address,
    #[serde(rename = "maxFeePerGas")]
    pub gas_fee_cap: U256,
    pub gas: U64,
    pub to: Address,
    pub value: U256,
    #[serde(rename = "input")]
    pub data: Bytes,
}

impl ArbitrumContractTx {
    fn rlp_encoded_fields_length(&self) -> usize {
        self.chain_id.length()
            + self.request_id.length()
            + self.from.length()
            + self.gas_fee_cap.length()
            + self.gas.length()
            + self.to.length()
            + self.value.length()
            + self.data.length()
    }

    fn rlp_encode_fields(&self, out: &mut dyn alloy_rlp::BufMut) {
        self.chain_id.encode(out);
        self.request_id.encode(out);
        self.from.encode(out);
        self.gas_fee_cap.encode(out);
        self.gas.encode(out);
        self.to.encode(out);
        self.value.encode(out);
        self.data.encode(out);
    }
}

impl TryFrom<UnknownTxEnvelopeWithSigner> for ArbitrumContractTx {
    type Error = eyre::Error;

    fn try_from(value: UnknownTxEnvelopeWithSigner) -> Result<Self, Self::Error> {
        let mut result: ArbitrumContractTx = parse_tx_fields(value.inner)?;
        result.from = value.signer;
        Ok(result)
    }
}

#[derive(Debug, Deserialize)]
pub struct ArbitrumRetryTx {
    #[serde(rename = "chainId")]
    pub chain_id: U64,
    pub nonce: U64,
    #[serde(skip_deserializing)]
    pub from: Address,
    #[serde(rename = "maxFeePerGas")]
    pub gas_fee_cap: U256,
    pub gas: U64,
    pub to: Address,
    pub value: U256,
    #[serde(rename = "input")]
    pub data: Bytes,
    #[serde(rename = "ticketId")]
    pub ticket_id: FixedBytes<32>,
    #[serde(rename = "refundTo")]
    pub refund_to: Address,
    #[serde(rename = "maxRefund")]
    pub max_refund: U256,
    #[serde(rename = "submissionFeeRefund")]
    pub submission_fee_refund: U256,
}

impl ArbitrumRetryTx {
    fn rlp_encoded_fields_length(&self) -> usize {
        self.chain_id.length()
            + self.nonce.length()
            + self.from.length()
            + self.gas_fee_cap.length()
            + self.gas.length()
            + self.to.length()
            + self.value.length()
            + self.data.length()
            + self.ticket_id.length()
            + self.refund_to.length()
            + self.max_refund.length()
            + self.submission_fee_refund.length()
    }

    fn rlp_encode_fields(&self, out: &mut dyn alloy_rlp::BufMut) {
        self.chain_id.encode(out);
        self.nonce.encode(out);
        self.from.encode(out);
        self.gas_fee_cap.encode(out);
        self.gas.encode(out);
        self.to.encode(out);
        self.value.encode(out);
        self.data.encode(out);
        self.ticket_id.encode(out);
        self.refund_to.encode(out);
        self.max_refund.encode(out);
        self.submission_fee_refund.encode(out);
    }
}

impl TryFrom<UnknownTxEnvelopeWithSigner> for ArbitrumRetryTx {
    type Error = eyre::Error;

    fn try_from(value: UnknownTxEnvelopeWithSigner) -> Result<Self, Self::Error> {
        let mut result: ArbitrumRetryTx = parse_tx_fields(value.inner)?;
        result.from = value.signer;
        Ok(result)
    }
}

#[derive(Debug, Deserialize)]
pub struct ArbitrumSubmitRetryableTx {
    #[serde(rename = "chainId")]
    pub chain_id: U64,
    #[serde(rename = "requestId")]
    pub request_id: FixedBytes<32>,
    #[serde(skip_deserializing)]
    pub from: Address,
    #[serde(rename = "l1BaseFee")]
    pub l1_base_fee: U256,
    #[serde(rename = "depositValue")]
    pub deposit_value: U256,
    #[serde(rename = "maxFeePerGas")]
    pub gas_fee_cap: U256,
    pub gas: U64,
    #[serde(rename = "retryTo")]
    pub retry_to: Address,
    #[serde(rename = "retryValue")]
    pub retry_value: U256,
    pub beneficiary: Address,
    #[serde(rename = "maxSubmissionFee")]
    pub max_submission_fee: U256,
    #[serde(rename = "refundTo")]
    pub fee_refund_addr: Address,
    #[serde(rename = "retryData")]
    pub retry_data: Bytes,
}

impl ArbitrumSubmitRetryableTx {
    fn rlp_encoded_fields_length(&self) -> usize {
        self.chain_id.length()
            + self.request_id.length()
            + self.from.length()
            + self.l1_base_fee.length()
            + self.deposit_value.length()
            + self.gas_fee_cap.length()
            + self.gas.length()
            + self.retry_to.length()
            + self.retry_value.length()
            + self.beneficiary.length()
            + self.max_submission_fee.length()
            + self.fee_refund_addr.length()
            + self.retry_data.length()
    }

    fn rlp_encode_fields(&self, out: &mut dyn alloy_rlp::BufMut) {
        self.chain_id.encode(out);
        self.request_id.encode(out);
        self.from.encode(out);
        self.l1_base_fee.encode(out);
        self.deposit_value.encode(out);
        self.gas_fee_cap.encode(out);
        self.gas.encode(out);
        self.retry_to.encode(out);
        self.retry_value.encode(out);
        self.beneficiary.encode(out);
        self.max_submission_fee.encode(out);
        self.fee_refund_addr.encode(out);
        self.retry_data.encode(out);
    }
}

impl TryFrom<UnknownTxEnvelopeWithSigner> for ArbitrumSubmitRetryableTx {
    type Error = eyre::Error;

    fn try_from(value: UnknownTxEnvelopeWithSigner) -> Result<Self, Self::Error> {
        let mut result: ArbitrumSubmitRetryableTx = parse_tx_fields(value.inner)?;
        result.from = value.signer;
        Ok(result)
    }
}

#[derive(Debug, Deserialize)]
pub struct ArbitrumInternalTx {
    #[serde(rename = "chainId")]
    pub chain_id: U64,
    #[serde(rename = "input")]
    pub data: Bytes,
}

impl ArbitrumInternalTx {
    fn rlp_encoded_fields_length(&self) -> usize {
        self.chain_id.length() + self.data.length()
    }

    fn rlp_encode_fields(&self, out: &mut dyn alloy_rlp::BufMut) {
        self.chain_id.encode(out);
        self.data.encode(out);
    }
}

impl TryFrom<UnknownTxEnvelopeWithSigner> for ArbitrumInternalTx {
    type Error = eyre::Error;

    fn try_from(value: UnknownTxEnvelopeWithSigner) -> Result<Self, Self::Error> {
        parse_tx_fields(value.inner)
    }
}

fn parse_tx_fields<T>(value: UnknownTxEnvelope) -> Result<T, eyre::Error>
where
    T: for<'a> Deserialize<'a>,
{
    let fields = value.inner.fields;
    let hash = &value.hash;
    fields
        .deserialize_into()
        .map_err(|err| eyre!("failed to parse fields tx {}: {}", hash, err))
}
