use bitcoin::OutPoint;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChannelBalance {
    pub initiator_sats: u64,
    pub counterparty_sats: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimulatedChannel {
    pub funding_outpoint: OutPoint,
    pub balance: ChannelBalance,
}

impl SimulatedChannel {
    pub fn new(funding_outpoint: OutPoint, balance: ChannelBalance) -> Self {
        Self {
            funding_outpoint,
            balance,
        }
    }
}
