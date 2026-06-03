use bitcoin::{Amount, ScriptBuf};
use lightning_payjoin_kit::psbt::FundingPsbtBuilder;

#[test]
fn builds_empty_fallback_psbt_with_channel_output() {
    let builder = FundingPsbtBuilder::new(Amount::from_sat(1_000_000), ScriptBuf::new());

    let psbt = builder.build_empty_fallback().expect("fallback PSBT");

    assert_eq!(psbt.unsigned_tx.output.len(), 1);
    assert_eq!(
        psbt.unsigned_tx.output[0].value,
        Amount::from_sat(1_000_000)
    );
}
