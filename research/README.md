# Research record — lightning-payjoin-kit

This folder is the project's research record. It exists because the library's
earlier documentation made claims that verification did not support, and because
the central question the project was built to answer now has an answer.

Read `02-the-central-result.md` first. It is the one that changes what the project
should do next.

Everything measured against real Bitcoin data is measured against real Bitcoin
data: 150,769 mainnet transactions parsed from raw consensus bytes, and a
collaborative funding transaction broadcast to and mined by Bitcoin Core. Where a
figure comes from a model rather than a measurement, it says so.

## Contents

| File | What it covers |
|---|---|
| `01-claims-corrections.md` | Nine claims from the earlier documentation, what verification found, and the sentence that replaces each |
| `02-the-central-result.md` | Whether the construction survives partitioning. It does not, and fixing the fee arithmetic does not change that |
| `03-construction-risks.md` | Six findings against the code, each with the correct construction and a citation for it |
| `04-open-questions.md` | The eight open questions, each with a recorded result including the negative ones |
| `05-composed-layers.md` | Taproot channels, splicing and transport: what exists today, at what version, with release evidence |
| `06-prior-art.md` | Dual funding, nolooking, and what this project adds that they do not |
| `07-sources.md` | Every source, with tier, citation and access date |
| `adversarial-harness/` | The partitioning harness, runnable with one command |
| `mainnet-baseline/` | 150,769 real mainnet transactions parsed from raw blocks, and the value distributions |
| `roundtrip-timing/` | Round-trip latency under concurrent load |
| `fee-arithmetic/` | The residue, weight estimation and marginal cost measurements |

## How claims are labelled

Every claim carries one of four labels, and none is promoted without new evidence.

- **VERIFIED** — supported by a specification, a peer-reviewed paper, pinned
  implementation source, or an experiment recorded here.
- **DISPUTED** — evidence points both ways, and both directions are shown.
- **UNVERIFIED** — not settled. The gap is stated rather than filled.
- **REFUTED** — the evidence contradicts the claim.

## The standard this record holds itself to

The project's own definition sets it: *the project does not claim, and its
documentation must never claim, that attribution becomes impossible; the claim is
that it becomes unreliable.* Nothing here should describe any construction as
anonymous, unlinkable, or mathematically impossible to attribute. Where a number
can be given, it is given. Where a question could not be settled, it says so.

Negative results are recorded in full. The most important result in this folder is
a negative one.
