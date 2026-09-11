# Experiment 06 — V7, collaborative round-trip under load

**Hypothesis.** The collaborative round-trip completes well inside LDK's
unfunded-channel window.

**Pass condition, stated before running.** The library's own compute must consume
under 1% of the window at the worst observed latency. The window is
`UNFUNDED_CHANNEL_AGE_LIMIT_TICKS = 60` ticks, which is about 3,600 s at LDK's
recommended once-per-minute cadence.

**Method.** All four protocol steps, driven through the library's public
`FundingCoordinator` API exactly as its own integration tests do:

1. `prepare_original` — initiator
2. `propose_privacy_input` — contributor
3. `validate_privacy_input_proposal` — initiator
4. `finalize_validated_proposal` — initiator

Timed end to end per session, run concurrently across 8 threads on a 2-CPU
machine — deliberately oversubscribed 4:1, so the measurement includes real
scheduling contention rather than a quiet single-threaded best case. 64 warm-up
sessions are discarded before timing begins. The crate under test is a path
dependency, compiled unmodified.

**Environment.** rustc 1.98.1, cargo 1.98.1, Linux 6.17.0-1022-azure, 2 CPUs,
release profile, 2026-09-10.

**Result: PASS.** 160,000 sessions, 8 threads, 1.147 s wall clock.

| | |
|---|---|
| mean | 24.7 µs |
| p50 | 3.3 µs |
| p90 | 3.5 µs |
| p99 | 4.7 µs |
| p99.9 | 10,019 µs |
| max | 123,997 µs |
| throughput | 139,465 sessions/s |

Against the window: worst observed compute 0.124 s, which is **0.0034%** of
3,600 s — a headroom factor of about 29,000×.

**The tail is scheduler noise, not protocol cost.** p50 to p99 spans 3.3–4.7 µs;
p99.9 jumps to 10 ms and the max to 124 ms. That is 8 threads contending for 2
CPUs, and the gap between p99 and p99.9 is where the thread was descheduled. Even
taking the worst figure at face value it is four orders of magnitude inside the
budget.

**Network is not measured; it is bounded.** Only compute was timed. The exchange
is one round-trip over an already-established connection — the initiator sends the
original, the contributor returns the proposal, and the initiator finishes locally.
Adding a round-trip time to the worst observed compute:

| RTT | total | share of window |
|---|---|---|
| 0.5 s | 0.6 s | 0.017% |
| 2 s | 2.1 s | 0.059% |
| 10 s | 10.1 s | 0.281% |
| 60 s | 60.1 s | 1.670% |
| 300 s | 300.1 s | 8.337% |

Even a contributor that takes five minutes to respond uses 8% of the window.

**What this settles.** V7 passes with very large margin. Timing is not a
constraint on the MVP path, and was never likely to be: two peers mid-handshake
on an open BOLT 8 connection have an hour to exchange two messages.

**What it does not settle.** No transport was measured — the RTT column is
arithmetic, not observation. It says nothing about a third-party contributor
reached over an undesigned rendezvous, which is U8 and remains open. It also does
not model a contributor requiring human approval. And the window itself is softer
than it looks: 60 ticks is an hour only at LDK's *recommended* cadence, which is
the integrator's choice, not a protocol constant (see U6).

**Reproduce.** `./run.sh`, or `SESSIONS=50000 THREADS=16 ./run.sh`
