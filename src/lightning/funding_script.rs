use bitcoin::{key::PublicKey, ScriptBuf};

pub fn p2wsh_2of2_funding_script(_a: PublicKey, _b: PublicKey) -> ScriptBuf {
    ScriptBuf::new()
}
