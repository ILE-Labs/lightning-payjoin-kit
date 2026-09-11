//! Experiment 06 — collaborative round-trip latency under concurrent load (V7).
//!
//! Measures the four protocol steps the library performs, end to end, under a
//! configurable number of concurrent sessions, and reports the distribution
//! against LDK's unfunded-channel window.
//!
//! The crate under test is a path dependency and is compiled unmodified.

use std::str::FromStr;
use std::sync::{Arc, Barrier};
use std::time::{Duration, Instant};

use bitcoin::{Amount, OutPoint, ScriptBuf, Txid};
use lightning_payjoin_kit::directory::MockDirectory;
use lightning_payjoin_kit::wallet::{MemoryWallet, Utxo};
use lightning_payjoin_kit::{
    FundingCoordinator, FundingMode, FundingPolicy, FundingRequest,
};

fn utxo(value_sats: u64, seed: u8, vout: u32) -> Utxo {
    let hex: String = std::iter::repeat(format!("{seed:02x}")).take(32).collect();
    Utxo {
        outpoint: OutPoint { txid: Txid::from_str(&hex).expect("txid"), vout },
        value: Amount::from_sat(value_sats),
        script_pubkey: ScriptBuf::new(),
        confirmed: true,
    }
}

fn request() -> FundingRequest {
    FundingRequest {
        channel_value_sats: 1_000_000,
        funding_script: ScriptBuf::new(),
        mode: FundingMode::PrivacyInput,
        fee_rate_sat_vb: 2.0,
        deadline: Duration::from_secs(30),
    }
}

/// One full collaborative round-trip: the four steps the two parties perform.
fn one_session(seed: u8) -> Duration {
    let policy = FundingPolicy::default();
    let mut initiator = FundingCoordinator::new(
        MemoryWallet::new(vec![utxo(1_100_000, seed, 0)], vec![ScriptBuf::new()]),
        MockDirectory::default(),
        policy.clone(),
    );
    let mut contributor = FundingCoordinator::new(
        MemoryWallet::new(vec![utxo(200_000, seed.wrapping_add(1), 0)], vec![ScriptBuf::new()]),
        MockDirectory::default(),
        policy,
    );
    let req = request();

    let t0 = Instant::now();
    let original = initiator.prepare_original(&req).expect("original");
    let proposal = contributor
        .propose_privacy_input(&original.psbt, &req)
        .expect("proposal");
    let _v = initiator
        .validate_privacy_input_proposal(&original.psbt, &proposal.psbt)
        .expect("validated");
    let result = initiator
        .finalize_validated_proposal(&original.psbt, proposal.psbt)
        .expect("finalized");
    let dt = t0.elapsed();
    assert_eq!(result.transaction.input.len(), 2);
    assert_eq!(result.transaction.output.len(), 3);
    dt
}

fn arg(name: &str, default: u64) -> u64 {
    let a: Vec<String> = std::env::args().collect();
    a.iter()
        .position(|x| x == name)
        .and_then(|i| a.get(i + 1))
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn percentile(sorted: &[u128], p: f64) -> u128 {
    if sorted.is_empty() {
        return 0;
    }
    let idx = ((sorted.len() - 1) as f64 * p).round() as usize;
    sorted[idx]
}

fn main() {
    let per_thread = arg("--sessions", 20_000) as usize;
    let threads = arg("--threads", 0) as usize;
    let threads = if threads == 0 {
        std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4)
    } else {
        threads
    };

    // LDK: UNFUNDED_CHANNEL_AGE_LIMIT_TICKS = 60 ticks, at the recommended
    // once-per-minute cadence.
    const WINDOW_SECS: u64 = 60 * 60;

    println!("# collaborative round-trip latency (V7)");
    println!("# threads         = {threads} concurrent sessions");
    println!("# sessions/thread = {per_thread}");
    println!("# total sessions  = {}", threads * per_thread);
    println!("# LDK window      = 60 ticks x ~60s = {WINDOW_SECS}s");
    println!();

    // Warm up, so the first-iteration allocator cost is not counted as latency.
    for s in 0..64u8 {
        one_session(s);
    }

    let barrier = Arc::new(Barrier::new(threads));
    let wall = Instant::now();
    let handles: Vec<_> = (0..threads)
        .map(|t| {
            let b = Arc::clone(&barrier);
            std::thread::spawn(move || {
                let mut v = Vec::with_capacity(per_thread);
                b.wait();
                for i in 0..per_thread {
                    let seed = (t as u8).wrapping_mul(37).wrapping_add(i as u8);
                    v.push(one_session(seed).as_nanos());
                }
                v
            })
        })
        .collect();

    let mut all: Vec<u128> = handles.into_iter().flat_map(|h| h.join().unwrap()).collect();
    let wall = wall.elapsed();
    all.sort_unstable();

    let n = all.len();
    let mean = all.iter().sum::<u128>() as f64 / n as f64;
    let us = |v: u128| v as f64 / 1000.0;

    println!("## Round-trip latency, all four protocol steps");
    println!("  sessions completed   {n}");
    println!("  mean                 {:>10.1} us", mean / 1000.0);
    println!("  p50                  {:>10.1} us", us(percentile(&all, 0.50)));
    println!("  p90                  {:>10.1} us", us(percentile(&all, 0.90)));
    println!("  p99                  {:>10.1} us", us(percentile(&all, 0.99)));
    println!("  p99.9                {:>10.1} us", us(percentile(&all, 0.999)));
    println!("  max                  {:>10.1} us", us(all[n - 1]));
    println!();
    println!("  wall clock           {:>10.3} s for {n} sessions across {threads} threads", wall.as_secs_f64());
    println!("  throughput           {:>10.0} sessions/s", n as f64 / wall.as_secs_f64());
    println!();

    let worst = all[n - 1] as f64 / 1e9;
    println!("## Against LDK's unfunded-channel window");
    println!("  window                       {WINDOW_SECS} s");
    println!("  worst observed compute       {worst:.6} s");
    println!("  headroom factor              {:.0}x", WINDOW_SECS as f64 / worst.max(1e-9));
    println!("  compute share of budget      {:.8}%", 100.0 * worst / WINDOW_SECS as f64);
    println!();
    println!("## Budget decomposition (compute measured; network NOT measured)");
    println!("  The exchange is two messages: original -> proposal, then the initiator");
    println!("  completes locally. Over an established BOLT 8 connection that is one");
    println!("  round-trip. Assuming a pessimistic Tor round-trip and a contributor that");
    println!("  responds only after a long delay:");
    for rtt in [0.5f64, 2.0, 10.0, 60.0, 300.0] {
        let total = rtt + worst;
        println!(
            "    RTT {rtt:>6.1}s  ->  total {:>7.1}s  = {:>6.3}% of the window",
            total,
            100.0 * total / WINDOW_SECS as f64
        );
    }
    println!();
    let pass = worst < WINDOW_SECS as f64 * 0.01;
    println!("## V7 gate");
    println!("  pass condition, stated before running: the library's own compute must");
    println!("  consume under 1% of the window at the worst observed latency.");
    println!("  result: {}", if pass { "PASS" } else { "FAIL" });
}
