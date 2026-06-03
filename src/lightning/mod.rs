mod funding_script;
mod simulated;

pub use funding_script::{p2wsh_2of2_funding_script, FundingScript};
pub use simulated::{validate_funding_transaction, ChannelBalance, SimulatedChannel};
