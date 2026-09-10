//! A minimal transaction model. Deliberately not a full Bitcoin transaction: the
//! heuristics under study operate on values, counts, script kinds and the two
//! metadata fields (nSequence, nLockTime), and nothing else.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Owner {
    Initiator,
    Contributor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptKind {
    P2wpkh,
    P2wsh,
    P2tr,
}

impl ScriptKind {
    /// Serialized TxOut size in bytes: 8 (value) + 1 (varint) + scriptPubKey.
    pub fn txout_vbytes(self) -> u64 {
        match self {
            ScriptKind::P2wpkh => 31, // 8 + 1 + 22
            ScriptKind::P2wsh => 43,  // 8 + 1 + 34
            ScriptKind::P2tr => 43,   // 8 + 1 + 34
        }
    }
}

#[derive(Debug, Clone)]
pub struct Input {
    pub value: u64,
    pub owner: Owner,
    pub sequence: u32,
}

#[derive(Debug, Clone)]
pub struct Output {
    pub value: u64,
    pub kind: ScriptKind,
    /// `None` marks the shared channel funding output, which belongs to neither
    /// party alone; the adversary is not told this and must infer it.
    pub owner: Option<Owner>,
}

#[derive(Debug, Clone)]
pub struct Tx {
    pub inputs: Vec<Input>,
    pub outputs: Vec<Output>,
    pub lock_time: u32,
    /// Ground truth, supplied by the generator and used only for scoring. The
    /// heuristics never read it.
    pub funding_index: usize,
    /// Retained so a reviewer extending the harness can compare the rate the
    /// construction targeted against the rate the finished transaction discloses.
    #[allow(dead_code)]
    pub target_feerate: u64,
}

impl Tx {
    /// Re-cast every output as P2TR, modelling a simple taproot channel where the
    /// funding output is an ordinary-looking single-signature output and the
    /// change outputs are taproot too. This is what an observer sees once
    /// `option_simple_taproot` channels are in use.
    pub fn as_taproot(&self) -> Tx {
        let mut t = self.clone();
        for o in t.outputs.iter_mut() {
            o.kind = ScriptKind::P2tr;
        }
        t
    }
}

impl Tx {
    pub fn fee(&self) -> u64 {
        let i: u64 = self.inputs.iter().map(|i| i.value).sum();
        let o: u64 = self.outputs.iter().map(|o| o.value).sum();
        i - o
    }

    /// Realistic vsize for P2WPKH-spending inputs.
    /// non-witness per input = 41 bytes; witness per input = 108 wu.
    /// overhead = 10 non-witness bytes + 2 wu segwit marker/flag.
    pub fn vsize(&self) -> u64 {
        let nonwit: u64 = 10
            + 41 * self.inputs.len() as u64
            + self.outputs.iter().map(|o| o.kind.txout_vbytes()).sum::<u64>();
        let wit: u64 = 2 + 108 * self.inputs.len() as u64;
        (nonwit * 4 + wit).div_ceil(4)
    }

    /// The fee rate an observer computes from the transaction itself.
    pub fn observed_feerate(&self) -> f64 {
        self.fee() as f64 / self.vsize() as f64
    }

    pub fn change_indices(&self) -> Vec<usize> {
        (0..self.outputs.len()).filter(|&i| i != self.funding_index).collect()
    }
}
