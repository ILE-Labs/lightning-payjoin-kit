#![cfg(feature = "corepc")]

use std::env;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use bitcoin::hex::FromHex;
use bitcoin::{
    key::{CompressedPublicKey, PrivateKey},
    secp256k1, Address, Amount, Network, NetworkKind, OutPoint, ScriptBuf, Txid,
};
use lightning_payjoin_kit::chain::{CorepcAuth, CorepcRegtestClient};
use lightning_payjoin_kit::directory::MockDirectory;
use lightning_payjoin_kit::lightning::{
    ChannelBalance, ChannelFundingHandoff, CommitmentSafety, FundingScript, PayjoinChannelFunder,
    SimulatedChannelFunder,
};
use lightning_payjoin_kit::wallet::{MemoryWallet, Utxo};
use lightning_payjoin_kit::{FundingCoordinator, FundingMode, FundingPolicy, FundingRequest};
use serde_json::Value;

#[test]
#[ignore = "requires docker compose bitcoind regtest node on 127.0.0.1:18443"]
fn broadcasts_collaborative_funding_transaction_on_bitcoin_core_regtest() {
    let base_url =
        env::var("LPK_COREPC_URL").unwrap_or_else(|_| "http://127.0.0.1:18443".to_owned());
    let auth = CorepcAuth::UserPass {
        user: env::var("LPK_COREPC_USER").unwrap_or_else(|_| "lpk".to_owned()),
        password: env::var("LPK_COREPC_PASSWORD").unwrap_or_else(|_| "lpk".to_owned()),
    };
    let base_rpc = CorepcRegtestClient::new(&base_url, auth.clone()).expect("base rpc");
    let wallet_name = unique_wallet_name();
    base_rpc.create_wallet(&wallet_name).expect("create wallet");

    let wallet_url = format!("{}/wallet/{wallet_name}", base_url.trim_end_matches('/'));
    let wallet_rpc = CorepcRegtestClient::new(&wallet_url, auth.clone()).expect("wallet rpc");
    let mining_address = wallet_rpc.new_address().expect("mining address");
    wallet_rpc
        .generate_to_address(101, &mining_address)
        .expect("mine mature wallet funds");

    let initiator_key = private_key(1);
    let counterparty_key = private_key(2);
    let initiator_address = p2wpkh_address(&initiator_key);
    let counterparty_address = p2wpkh_address(&counterparty_key);

    let initiator_fund_txid = wallet_rpc
        .send_to_address(&initiator_address, Amount::from_sat(1_100_000))
        .expect("fund initiator input");
    let counterparty_fund_txid = wallet_rpc
        .send_to_address(&counterparty_address, Amount::from_sat(200_000))
        .expect("fund counterparty input");
    let confirmation_address = wallet_rpc.new_address().expect("confirmation address");
    wallet_rpc
        .generate_to_address(1, &confirmation_address)
        .expect("confirm peer inputs");

    let initiator_utxo = funded_utxo(
        &wallet_rpc,
        initiator_fund_txid,
        &initiator_address,
        Amount::from_sat(1_100_000),
    );
    let counterparty_utxo = funded_utxo(
        &wallet_rpc,
        counterparty_fund_txid,
        &counterparty_address,
        Amount::from_sat(200_000),
    );
    let initiator_outpoint = initiator_utxo.outpoint;
    let counterparty_outpoint = counterparty_utxo.outpoint;

    let secp = secp256k1::Secp256k1::new();
    let funding_script = FundingScript::new_2of2(
        private_key(11).public_key(&secp),
        private_key(12).public_key(&secp),
    );
    let request = FundingRequest {
        channel_value_sats: 1_000_000,
        funding_script: funding_script.script_pubkey.clone(),
        mode: FundingMode::PrivacyInput,
        fee_rate_sat_vb: 2.0,
        deadline: Duration::from_secs(30),
    };
    let policy = FundingPolicy::default();

    let mut initiator = FundingCoordinator::new(
        MemoryWallet::new_with_keys(
            vec![initiator_utxo],
            vec![initiator_address.script_pubkey()],
            vec![(initiator_outpoint, initiator_key)],
        ),
        MockDirectory::default(),
        policy.clone(),
    );
    let mut counterparty = FundingCoordinator::new(
        MemoryWallet::new_with_keys(
            vec![counterparty_utxo],
            vec![counterparty_address.script_pubkey()],
            vec![(counterparty_outpoint, counterparty_key)],
        ),
        MockDirectory::default(),
        policy,
    );

    let original = initiator.prepare_original(&request).expect("original");
    let proposal = counterparty
        .propose_privacy_input(&original.psbt, &request)
        .expect("proposal");
    let result = initiator
        .finalize_validated_proposal(&original.psbt, proposal.psbt)
        .expect("finalized funding result");

    let handoff = ChannelFundingHandoff::new(
        result.clone(),
        funding_script.script_pubkey,
        ChannelBalance {
            initiator_sats: 1_000_000,
            counterparty_sats: 0,
        },
        FundingMode::PrivacyInput,
        CommitmentSafety::CommitmentsExchanged,
    );
    let mut channel_funder = SimulatedChannelFunder;
    let channel = channel_funder
        .accept_funding(handoff)
        .expect("simulated channel from live funding transaction");
    assert_eq!(channel.funding_outpoint, result.funding_outpoint);

    let mut broadcaster = CorepcRegtestClient::new(&base_url, auth).expect("broadcast rpc");
    let broadcast_txid = initiator
        .broadcast_funding(&result, &mut broadcaster)
        .expect("broadcast funding transaction");
    assert_eq!(broadcast_txid, result.transaction.compute_txid());

    let block_address = wallet_rpc.new_address().expect("block address");
    wallet_rpc
        .generate_to_address(1, &block_address)
        .expect("mine broadcast funding transaction");
}

fn unique_wallet_name() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_millis();
    format!("lpk-regtest-{millis}")
}

fn private_key(secret_byte: u8) -> PrivateKey {
    PrivateKey::new(
        secp256k1::SecretKey::from_slice(&[secret_byte; 32]).expect("secret key"),
        NetworkKind::Test,
    )
}

fn p2wpkh_address(private_key: &PrivateKey) -> Address {
    let secp = secp256k1::Secp256k1::new();
    let public_key =
        CompressedPublicKey::from_private_key(&secp, private_key).expect("compressed public key");
    Address::p2wpkh(&public_key, Network::Regtest)
}

fn funded_utxo(rpc: &CorepcRegtestClient, txid: Txid, address: &Address, value: Amount) -> Utxo {
    Utxo {
        outpoint: funded_outpoint(txid, funded_vout(rpc, txid, address)),
        value,
        script_pubkey: address.script_pubkey(),
        confirmed: true,
    }
}

fn funded_outpoint(txid: Txid, vout: u32) -> OutPoint {
    OutPoint { txid, vout }
}

fn funded_vout(rpc: &CorepcRegtestClient, txid: Txid, address: &Address) -> u32 {
    let tx = rpc
        .get_wallet_transaction(txid)
        .expect("wallet transaction");
    let vouts = tx
        .pointer("/decoded/vout")
        .and_then(Value::as_array)
        .expect("decoded vouts");

    vouts
        .iter()
        .find_map(|vout| {
            let script_hex = vout.pointer("/scriptPubKey/hex")?.as_str()?;
            let script_bytes = Vec::<u8>::from_hex(script_hex).ok()?;
            let script = ScriptBuf::from_bytes(script_bytes);
            if script == address.script_pubkey() {
                let n = vout.get("n")?.as_u64()?;
                Some(n as u32)
            } else {
                None
            }
        })
        .expect("funding output vout")
}
