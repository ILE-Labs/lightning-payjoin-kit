use bitcoin::{
    ecdsa, key::PrivateKey, secp256k1, sighash::EcdsaSighashType, sighash::SighashCache, Amount,
    NetworkKind, OutPoint, Psbt, ScriptBuf, Transaction, TxOut, Witness,
};

use crate::error::{Error, Result};
use crate::wallet::{SigningSummary, Utxo, Wallet};

#[derive(Debug, Clone, Default)]
pub struct MemoryWallet {
    utxos: Vec<Utxo>,
    change_scripts: Vec<ScriptBuf>,
    keys: Vec<OwnedKey>,
}

#[derive(Debug, Clone)]
struct OwnedKey {
    outpoint: OutPoint,
    private_key: PrivateKey,
}

impl MemoryWallet {
    pub fn new(utxos: Vec<Utxo>, change_scripts: Vec<ScriptBuf>) -> Self {
        Self {
            utxos,
            change_scripts,
            keys: Vec::new(),
        }
    }

    pub fn new_with_keys(
        utxos: Vec<Utxo>,
        change_scripts: Vec<ScriptBuf>,
        keys: Vec<(OutPoint, PrivateKey)>,
    ) -> Self {
        Self {
            utxos,
            change_scripts,
            keys: keys
                .into_iter()
                .map(|(outpoint, private_key)| OwnedKey {
                    outpoint,
                    private_key,
                })
                .collect(),
        }
    }

    pub fn deterministic_p2wpkh(
        outpoint: OutPoint,
        value: Amount,
        secret_byte: u8,
        change_scripts: Vec<ScriptBuf>,
    ) -> Result<Self> {
        let private_key = deterministic_private_key(secret_byte)?;
        let secp = secp256k1::Secp256k1::new();
        let public_key = private_key.public_key(&secp);
        let wpubkey_hash = public_key
            .wpubkey_hash()
            .map_err(|err| Error::Wallet(err.to_string()))?;
        let script_pubkey = ScriptBuf::new_p2wpkh(&wpubkey_hash);
        let utxo = Utxo {
            outpoint,
            value,
            script_pubkey,
            confirmed: true,
        };

        Ok(Self::new_with_keys(
            vec![utxo],
            change_scripts,
            vec![(outpoint, private_key)],
        ))
    }
}

impl Wallet for MemoryWallet {
    fn list_spendable_utxos(&self) -> Result<Vec<Utxo>> {
        Ok(self.utxos.clone())
    }

    fn next_change_script(&mut self) -> Result<ScriptBuf> {
        Ok(self.change_scripts.pop().unwrap_or_default())
    }

    fn sign_owned_inputs(&self, _transaction: &mut Transaction) -> Result<()> {
        Ok(())
    }

    fn sign_owned_psbt(&self, psbt: &mut Psbt) -> Result<SigningSummary> {
        if self.keys.is_empty() {
            let signed_inputs = psbt
                .unsigned_tx
                .input
                .iter()
                .filter(|input| {
                    self.utxos
                        .iter()
                        .any(|utxo| utxo.outpoint == input.previous_output)
                })
                .count();

            return Ok(SigningSummary { signed_inputs });
        }

        let tx = psbt.unsigned_tx.clone();
        let secp = secp256k1::Secp256k1::new();
        let mut signed_inputs = 0;

        for input_index in 0..psbt.unsigned_tx.input.len() {
            let previous_output = psbt.unsigned_tx.input[input_index].previous_output;
            let Some(owned_key) = self
                .keys
                .iter()
                .find(|owned_key| owned_key.outpoint == previous_output)
            else {
                continue;
            };
            let witness_utxo = psbt.inputs[input_index]
                .witness_utxo
                .clone()
                .ok_or_else(|| Error::InvalidPsbt("owned input missing witness_utxo".to_owned()))?;
            let public_key = owned_key.private_key.public_key(&secp);
            ensure_p2wpkh_matches_key(&witness_utxo, &public_key)?;

            let mut cache = SighashCache::new(&tx);
            let sighash = cache
                .p2wpkh_signature_hash(
                    input_index,
                    &witness_utxo.script_pubkey,
                    witness_utxo.value,
                    EcdsaSighashType::All,
                )
                .map_err(|err| Error::Signing(err.to_string()))?;
            let message = secp256k1::Message::from(sighash);
            let signature = secp.sign_ecdsa(&message, &owned_key.private_key.inner);
            let signature = ecdsa::Signature {
                signature,
                sighash_type: EcdsaSighashType::All,
            };

            psbt.inputs[input_index].final_script_witness =
                Some(Witness::p2wpkh(&signature, &public_key.inner));
            signed_inputs += 1;
        }

        Ok(SigningSummary { signed_inputs })
    }
}

fn deterministic_private_key(secret_byte: u8) -> Result<PrivateKey> {
    let secret_byte = secret_byte.max(1);
    let secret_key = secp256k1::SecretKey::from_slice(&[secret_byte; 32])
        .map_err(|err| Error::Wallet(err.to_string()))?;
    Ok(PrivateKey::new(secret_key, NetworkKind::Test))
}

fn ensure_p2wpkh_matches_key(txout: &TxOut, public_key: &bitcoin::PublicKey) -> Result<()> {
    let expected = ScriptBuf::new_p2wpkh(
        &public_key
            .wpubkey_hash()
            .map_err(|err| Error::Signing(err.to_string()))?,
    );

    if txout.script_pubkey != expected {
        return Err(Error::Signing(
            "owned input script does not match signing key".to_owned(),
        ));
    }

    if !txout.script_pubkey.is_p2wpkh() {
        return Err(Error::Signing(
            "memory wallet only signs p2wpkh inputs".to_owned(),
        ));
    }

    Ok(())
}
