# Claims corrections

Nine claims appeared in this project's earlier documentation that verification did
not support. Each is listed with the text as it stood, what was found, and what
replaces it. They are kept rather than quietly deleted, because a reader who saw
the old text deserves to know what happened to it.

Line references are to the repository before the correction.

---

### 1. "Asynchronous Rust library" / "async coordination engine"

**Found:** REFUTED. There is no `async`, `await`, `tokio` or `Future` anywhere in
`src/` or `tests/`. `DirectoryClient` is a synchronous trait; the mock directory
holds `Rc<RefCell<_>>` and is neither `Send` nor `Sync`. The quick-start example
in the old README did not compile — it named types the crate does not export.

**Replaces it:** A synchronous Rust library. Coordination is a turn-based exchange
of PSBTs with no runtime and no I/O of its own; the caller drives it.

---

### 2. "Zero-dependency core"

**Found:** REFUTED. Five mandatory dependencies — `bitcoin`, `secp256k1`, `serde`,
`serde_json`, `thiserror` — plus optional `corepc-client` and `lightning`.
`serde_json` is not a Bitcoin library and is not optional.

**Replaces it:** Five mandatory dependencies, listed. Bitcoin Core RPC and LDK are
optional and feature-gated.

---

### 3. "BIP-78 native, implements the Payjoin standard"

**Found:** REFUTED. Neither the BIP-78 nor the BIP-77 wire format is implemented:
no HTTP endpoint, no BIP21 `pj=` parameter, no `maxadditionalfeecontribution` or
`minfeerate`, none of the BIP-78 error codes. Messages are serialized PSBTs tagged
with a payload kind, exchanged through an in-memory map.

**Replaces it:** Payjoin-inspired, not BIP-78. The protocol shape is borrowed from
BIP-78; the encoding is this project's own.

---

### 4. "We implemented an AES-256-GCM encryption layer… session keys derived via ECDH"

**Found:** REFUTED. There is no cryptography in the crate beyond what `bitcoin` and
`secp256k1` provide for transaction handling. No key exchange, no AEAD, no session
keys. Searching the tree for `aes`, `gcm`, `ecdh`, `encrypt`, `hpke`, `chacha`,
`poly1305` and `nonce` returns nothing.

This is the most serious of the nine. The others overstate the state of the work;
this one described a component that does not exist, in the past tense.

**Replaces it:** The claim is withdrawn. Transport is out of scope. The MVP path
carries proposals over the existing authenticated, encrypted BOLT 8 connection
between the two channel peers, so the library adds no cryptography of its own.

---

### 5. "Mathematically impossible for outside observers to determine which inputs funded the channel"

**Found:** REFUTED, twice over.

Against the code: the contributor's change is their input minus exactly
`99 x feerate`, so the difference is an exact multiple an observer can invert. This
held in 59 of 59 constructed transactions across fee rates from 1 to 300 sat/vB,
and the corresponding attack identifies the contributor's pair in 99.997% of 20,000
generated transactions, against a 50% chance baseline.

Against the construction: with every planned fix applied, subset-sum partitioning
still succeeds 99.85% of the time. See `02-the-central-result.md`.

**Replaces it:** Collaborative construction is intended to reduce an observer's
confidence in attributing funding inputs, not to make attribution impossible. As
currently built it does not achieve even that. No configuration of this library
should be described as a proof of unlinkability.

---

### 6. "No implementation exists for Lightning channel funding, until now"

**Found:** REFUTED. BOLT 2 channel establishment v2 has both peers contribute
inputs, and is live in Core Lightning and Eclair and implemented in LDK. nolooking
has opened Lightning channels from inbound BIP-78 payjoins since 2022. The idea was
discussed publicly by a payjoin maintainer in 2023.

**Replaces it:** see `06-prior-art.md`, which states what this project adds in one
sentence and what that is worth in the next.

---

### 7. "No extra fees"

**Found:** REFUTED, with the figure verified exactly. The construction adds exactly
99 vbytes for a contributor spending one P2WPKH input to one P2WPKH output: 41
non-witness bytes for the input, 31 for the output, 108 weight units of witness,
`(41 + 31) x 4 + 108 = 396` weight units, `396 / 4 = 99` vbytes. Measured by
serializing both transactions: fallback 153 vbytes, collaborative 252.

**Replaces it:** The construction adds exactly 99 vbytes, paid by the contributor.
99 sat at 1 sat/vB; 9,900 sat at 100 sat/vB.

---

### 8. "End-to-end testing with LDK nodes on Regtest", marked complete

**Found:** REFUTED. Two separate tests exist and neither is the one claimed. The
two-node LDK harness runs in-process with no chain backend. The regtest test
broadcasts to a real `bitcoind` with no Lightning node involved. Both report zero
tests on the default feature set.

Both have since been run under their own feature flags and both pass — two real
`ChannelManager`s reach a persisted channel, and a collaborative funding
transaction is mined by Bitcoin Core 28.0 on regtest. That confirms each test does
what it says; it does not create the intersection the claim describes.

**Replaces it:** The two are described separately, and the gap between them — LDK
channel managers whose funding transaction is confirmed by a real chain backend —
is stated as not yet covered.

---

### 9. `cargo install lightning-payjoin-kit --features cli`, and conflicting status tables

**Found:** REFUTED, on both halves. crates.io returns HTTP 404 for this crate name
and docs.rs returns 404, while a control request succeeds. There is no `cli`
feature in `Cargo.toml` and no binary target in the tree, so neither the
`cargo install` line nor the `= "0.1"` dependency line can work. Separately, the
README described M1 as in progress and M2 as planned while the roadmap described
both corresponding phases as PoC-complete.

**Replaces it:** The crate is not published and there is no command-line tool.
Build from source. One status table, in one place.

---

## A note on the claims that were true

Several claims in the earlier documentation held up under checking, and it is worth
saying so plainly.

The test suite passes exactly as described — 33 tests, no failures, no warnings.
The LDK APIs the integration uses are real and are called in the documented order.
Two real LDK `ChannelManager` instances do accept a collaboratively constructed
funding outpoint and reach a usable channel. A transaction built by this library
is accepted and mined by Bitcoin Core. And the threat the project describes is
real: Kappos et al. identified at least one participant in 86.8% of
private-channel opens using on-chain coin flow alone.

The mechanism works. What the measurements refute is the privacy claim made for
it — which is a narrower and more useful thing to have established than either
"it works" or "it doesn't".
