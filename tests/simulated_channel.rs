use bitcoin::{OutPoint, ScriptBuf, Txid};
use lightning_payjoin_kit::lightning::{ChannelBalance, SimulatedChannel};
use std::str::FromStr;

#[test]
fn simulated_channel_keeps_explicit_balances() {
    let channel = SimulatedChannel::new(
        OutPoint {
            txid: Txid::from_str(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .expect("zero txid"),
            vout: 0,
        },
        ChannelBalance {
            initiator_sats: 1_000_000,
            counterparty_sats: 0,
        },
        ScriptBuf::new(),
    );

    assert_eq!(channel.balance.counterparty_sats, 0);
}
