# Sources

Everything this record relies on. All accessed 2026-09-10.

## Specifications

- **BIP-78: A Simple Payjoin Proposal.**
  <https://github.com/bitcoin/bips/blob/master/bip-0078.mediawiki>
- **BOLT 2: Peer Protocol for Channel Management.** lightning/bolts at commit
  `152897261850d93c4f4597f39cf22d7d22d6ede6`, `02-peer-protocol.md`.
  <https://github.com/lightning/bolts>
- **Extension BOLT XX: Simple Taproot Channels.** Olaoluwa Osuntokun, Eugene
  Siegel, created 2022-04-20. `bolt-simple-taproot.md` in lightning/bolts at the
  same commit. *Draft: the file carries no assigned BOLT number and is not one of
  the numbered BOLTs.*
- **lightning/bolts PR #1160, "Channel Splicing (feature 62/63)."** Merged
  2026-03-23. <https://github.com/lightning/bolts/pull/1160>

## Academic

- **Ghesmati, S., Kern, A., Judmayer, A., Stifter, N., Weippl, E.,
  *Unnecessary Input Heuristics & PayJoin Transactions*.** IACR ePrint 2022/589,
  2022-05-17; the PDF states "This work published on HCI International 2021".
  <https://eprint.iacr.org/2022/589>
  *Note: the eprint landing page lists four authors; the PDF lists five. The PDF is
  taken as authoritative.*
- **Kappos, G., Yousaf, H., Piotrowska, A., Kanjalkar, S., Delgado-Segura, S.,
  Miller, A., Meiklejohn, S., *An Empirical Analysis of Privacy in the Lightning
  Network*.** arXiv:2003.12470 [cs.CR], v3 2021-01-21.
  <https://arxiv.org/abs/2003.12470>
- **Delgado-Segura, S., Pérez-Solà, C., Navarro-Arribas, G.,
  Herrera-Joancomartí, J., *Analysis of the Bitcoin UTXO set*.** IACR ePrint
  2017/1095; FC18 Bitcoin Workshop. <https://eprint.iacr.org/2017/1095>
  *Snapshot: 26 October 2017. Cited only for the definitions of dust and
  unprofitable outputs and their relation to fee rates.*

## Implementation source, pinned

- **rust-lightning (LDK) v0.2.2**, commit
  `0695da995fbe2810e11fd8824dd43eece994d111`.
  `lightning/src/ln/channel.rs`, `lightning/src/ln/channelmanager.rs`.
- **Bitcoin Core v31.1**, commit `9be056a8a72b624dae9623b2f7bded92c2a21c91`.
  `src/wallet/spend.cpp`, `src/primitives/transaction.h`, `src/util/rbf.h`.
- **Core Lightning v26.06.** `doc/schemas/fundchannel_start.json`,
  `doc/schemas/fundchannel_complete.json`.
- **LND v0.21.0-beta.** `lnrpc/lightning.proto`;
  `docs/release-notes/release-notes-0.21.0.md`.
- **payjoin/nolooking**, created 2022-07-24, last commit `ddb170ac89e5`
  2023-06-19. <https://github.com/payjoin/nolooking>

## Practitioner discourse

- **Bitcoin Optech Newsletter #131**, 2021-01-13, "LN dual funding anti UTXO
  probing". <https://bitcoinops.org/en/newsletters/2021/01/13/>
- **Dan Gould, "Interactive Payment Batching is Better"**, payjoin.org,
  2023-05-09.
  <https://payjoin.org/blog/2023/05/09/interactive-payment-batching-is-better/>
- **Bitcoin Optech topic page, "Fee sniping".**
  <https://bitcoinops.org/en/topics/fee-sniping/>

## Release evidence

Publisher `published_at` values, from each project's release metadata.

| Project | Tag | Published |
|---|---|---|
| LND | `v0.21.0-beta` | 2026-06-05 |
| Eclair | `v0.14.0` | 2026-05-21 |
| Core Lightning | `v26.04` | 2026-04-20 |
| Core Lightning | `v26.06` | 2026-06-02 |
| LDK | `v0.2` | 2025-12-02 (changelog header) |
| Bitcoin Core | `v31.1` | 2026-07-08 |

## Searches that found nothing

Recorded so they can be rerun.

| Search | Corpus | Result |
|---|---|---|
| `grep -rniE 'coinjoin\|coin.join' --include='*.md'` | lightning/bolts, all 16 markdown files, at the pinned commit | no matches |
| `grep -niE 'taproot'` | LND `release-notes-0.21.0.md` — for whether announced taproot channels are supported | matches confirm they are not |
| `GET crates.io/api/v1/crates/lightning-payjoin-kit` | crates.io | HTTP 404; control request for `bitcoin` returns 200 |

## Sources not verified

Stated so nobody mistakes them for checked facts.

- The DOI `10.1007/978-3-030-78642-7_56`, reported for Ghesmati et al. by the
  eprint landing page, was not independently resolved.
- Lloyd Fournier's lightning-dev post on anti-probing proposals was not opened;
  only Optech's account of it was read.
- BIP-78's author list was not confirmed against the document header.
- Whether Eclair permits announcing taproot channels was not checked in Eclair's
  own source or documentation.
