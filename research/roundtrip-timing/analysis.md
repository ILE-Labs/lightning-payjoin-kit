# Experiment 06 — analysis

## What it means

The library's own work is free relative to the budget. A 3.3 µs median against a
3,600 s window is not a margin to manage; it is a non-issue. Any timing risk in
this protocol lives entirely in the transport and in the contributor's willingness
to answer, not in the construction.

That has a design consequence worth stating. Because compute is free, there is no
performance argument against making the construction more expensive — random
shuffling, extra weight calculation, a more careful coin selection pass, or the
bond-and-verify schemes that R-P5 discusses. Whatever fixes the privacy problem,
its cost will not be measured here.

## What it does not mean

**It is not a measurement of the deployed system.** There is no network, no
serialisation over a real link, no peer that might be slow, offline or hostile.
The RTT table is arithmetic on top of the measured compute, not an observation.

**It does not validate the window.** U6 establishes the window is 60 *ticks*, and
that the tick cadence is chosen by the integrator. An integrator ticking every 10
seconds has a 10-minute window, and the RTT table should then be read against 600
s rather than 3,600.

**It says nothing about the third-party path.** U8's timing question is about
finding and reaching a stranger, which this does not touch.

## Threats to validity

**The mock directory is in-memory.** `MockDirectory` is a `BTreeMap` behind
`Rc<RefCell<_>>`; the sessions here never serialise a payload or cross a process
boundary. Real transport adds serialisation and I/O that this does not capture,
though both are small next to any plausible RTT.

**Both coordinators run in one thread per session.** A real deployment has the two
parties on separate machines, so the measured figure is the sum of two parties'
compute, which is the right total but not the right per-party latency.

**A 2-CPU machine with 8 threads is a specific kind of load.** It captures
scheduling contention well and captures nothing about memory pressure, cache
behaviour under a larger working set, or a node doing Lightning work at the same
time.

**Warm-up discards the first 64 sessions.** That removes allocator start-up cost,
which is the right call for steady-state latency and would understate the very
first session a freshly started node performs.
