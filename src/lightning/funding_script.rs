use bitcoin::{
    blockdata::{opcodes::all::OP_CHECKMULTISIG, script::Builder},
    key::PublicKey,
    ScriptBuf,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FundingScript {
    pub witness_script: ScriptBuf,
    pub script_pubkey: ScriptBuf,
}

impl FundingScript {
    pub fn new_2of2(a: PublicKey, b: PublicKey) -> Self {
        let (first, second) = sorted_pubkeys(a, b);
        let witness_script = Builder::new()
            .push_int(2)
            .push_key(&first)
            .push_key(&second)
            .push_int(2)
            .push_opcode(OP_CHECKMULTISIG)
            .into_script();
        let script_pubkey = witness_script.to_p2wsh();

        Self {
            witness_script,
            script_pubkey,
        }
    }
}

pub fn p2wsh_2of2_funding_script(a: PublicKey, b: PublicKey) -> ScriptBuf {
    FundingScript::new_2of2(a, b).script_pubkey
}

fn sorted_pubkeys(a: PublicKey, b: PublicKey) -> (PublicKey, PublicKey) {
    if a.inner.serialize() <= b.inner.serialize() {
        (a, b)
    } else {
        (b, a)
    }
}
