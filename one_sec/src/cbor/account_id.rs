use ic_ledger_types::AccountIdentifier;
use minicbor::decode::{Decoder, Error};
use minicbor::encode::{Encoder, Write};

pub fn decode<Ctx>(d: &mut Decoder<'_>, _ctx: &mut Ctx) -> Result<AccountIdentifier, Error> {
    let bytes = d.bytes()?;
    AccountIdentifier::from_slice(bytes).map_err(|e| Error::message(e.to_string()))
}

pub fn encode<Ctx, W: Write>(
    v: &AccountIdentifier,
    e: &mut Encoder<W>,
    _ctx: &mut Ctx,
) -> Result<(), minicbor::encode::Error<W::Error>> {
    e.bytes(v.as_bytes())?;
    Ok(())
}
