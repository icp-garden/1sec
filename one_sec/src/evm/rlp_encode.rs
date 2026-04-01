//! This module defines RLP encoding helpers for the EVM-related types.

use ethnum::u256;
use rlp::RlpStream;

use super::tx::{
    AccessList, AccessListItem, Eip1559Signature, Eip1559TransactionRequest,
    InnerSignedEip1559TransactionRequest, SignedEip1559TransactionRequest,
};

impl rlp::Encodable for AccessList {
    fn rlp_append(&self, s: &mut RlpStream) {
        s.append_list(&self.0);
    }
}

impl rlp::Encodable for AccessListItem {
    fn rlp_append(&self, s: &mut RlpStream) {
        const ACCESS_FIELD_COUNT: usize = 2;

        s.begin_list(ACCESS_FIELD_COUNT);
        s.append(&self.address.as_ref());
        s.begin_list(self.storage_keys.len());
        for storage_key in self.storage_keys.iter() {
            s.append(&storage_key.0.as_ref());
        }
    }
}

impl rlp::Encodable for Eip1559TransactionRequest {
    fn rlp_append(&self, s: &mut RlpStream) {
        s.begin_unbounded_list();
        rlp_inner(self, s);
        s.finalize_unbounded_list();
    }
}

impl rlp::Encodable for Eip1559Signature {
    fn rlp_append(&self, s: &mut RlpStream) {
        s.append(&self.signature_y_parity);
        encode_u256(s, self.r);
        encode_u256(s, self.s);
    }
}

impl rlp::Encodable for InnerSignedEip1559TransactionRequest {
    fn rlp_append(&self, s: &mut RlpStream) {
        s.begin_unbounded_list();
        rlp_inner(&self.transaction, s);
        s.append(&self.signature);
        //ignore memoized_hash
        s.finalize_unbounded_list();
    }
}

impl rlp::Encodable for SignedEip1559TransactionRequest {
    fn rlp_append(&self, s: &mut RlpStream) {
        s.append(&self.inner);
    }
}

fn rlp_inner(req: &Eip1559TransactionRequest, rlp: &mut RlpStream) {
    rlp.append(&req.chain_id);
    rlp.append(&req.nonce);
    rlp.append(&req.max_priority_fee_per_gas);
    rlp.append(&req.max_fee_per_gas);
    rlp.append(&req.gas_limit);
    rlp.append(&req.destination.as_ref());
    rlp.append(&req.amount);
    rlp.append(&req.data);
    rlp.append(&req.access_list);
}

pub fn encode_u256<T: Into<u256>>(stream: &mut RlpStream, value: T) {
    let value = value.into();
    let leading_empty_bytes: usize = value.leading_zeros() as usize / 8;
    stream.append(&value.to_be_bytes()[leading_empty_bytes..].as_ref());
}
