"""Structural baseline: what do ordinary mainnet transactions actually look like?

Answers U3/V5 by measuring the fields the library sets, over every non-coinbase
transaction in the sampled blocks, and then locating the library's own output in
that distribution.
"""
import json, sys, collections, os
sys.path.insert(0, os.path.dirname(__file__))
from parse_block import parse_block

SEQ_FINAL = 0xFFFFFFFF
SEQ_NONFINAL = 0xFFFFFFFE
SEQ_RBF = 0xFFFFFFFD


def seq_label(s):
    if s == SEQ_FINAL:
        return "0xffffffff (final)"
    if s == SEQ_NONFINAL:
        return "0xfffffffe (non-final)"
    if s == SEQ_RBF:
        return "0xfffffffd (BIP125 RBF)"
    if s < SEQ_RBF:
        return "< 0xfffffffd (other RBF)"
    return "other"


def pct(a, b):
    return 100.0 * a / b if b else 0.0


def main():
    man = json.load(open("raw/manifest.json"))

    n = 0
    seq_vals = collections.Counter()
    tx_all_final = 0
    tx_any_rbf = 0
    lock_zero = 0
    lock_height_near = 0
    lock_height_far = 0
    lock_other = 0
    nout_dist = collections.Counter()
    nin_dist = collections.Counter()
    ver_dist = collections.Counter()
    outtype_dist = collections.Counter()
    joint_lib = 0            # locktime==0 AND all sequences final
    joint_rbf_lt0 = 0        # all sequences RBF-signalling AND locktime==0
    joint_rbf_afs = 0        # all sequences RBF-signalling AND anti-fee-sniping locktime
    joint_fd_lt0 = 0         # all sequences exactly 0xfffffffd AND locktime==0
    shape_2in3out = 0
    shape_2in3out_lib = 0    # + locktime 0 + all final
    shape_2in3out_lib_scripts = 0  # + exactly one P2WSH and two P2WPKH outputs
    has_p2wsh_out = 0
    p2wsh_single_2out = 0
    n_out_le2 = 0
    mixed_seq = 0

    for b in man["blocks"]:
        data = open(f"raw/blocks/{b['height']}.blk", "rb").read()
        hdr, txs, consumed = parse_block(data)
        tip = b["height"]  # each tx is judged against the height of ITS OWN block
        assert consumed == len(data), f"parser desync at {b['height']}"
        assert len(txs) == b["tx_count"], f"tx count mismatch at {b['height']}"
        for t in txs:
            if t["coinbase"]:
                continue
            n += 1
            ver_dist[t["version"]] += 1
            nin_dist[min(t["n_in"], 20)] += 1
            nout_dist[min(t["n_out"], 20)] += 1
            for s in t["sequences"]:
                seq_vals[seq_label(s)] += 1
            allf = all(s == SEQ_FINAL for s in t["sequences"])
            anyrbf = any(s < SEQ_NONFINAL for s in t["sequences"])
            if allf:
                tx_all_final += 1
            if anyrbf:
                tx_any_rbf += 1
            if len(set(t["sequences"])) > 1:
                mixed_seq += 1
            lt = t["locktime"]
            if lt == 0:
                lock_zero += 1
            elif lt < 500_000_000 and (tip - 100) <= lt <= tip:
                lock_height_near += 1
            elif lt < 500_000_000:
                lock_height_far += 1
            else:
                lock_other += 1
            allrbf = all(sq < SEQ_NONFINAL for sq in t["sequences"])
            allfd = all(sq == SEQ_RBF for sq in t["sequences"])
            near = (lt != 0 and lt < 500_000_000 and (tip - 100) <= lt <= tip)
            if lt == 0 and allf:
                joint_lib += 1
            if lt == 0 and allrbf:
                joint_rbf_lt0 += 1
            if near and allrbf:
                joint_rbf_afs += 1
            if lt == 0 and allfd:
                joint_fd_lt0 += 1
            types = [k for _, k in t["outputs"]]
            for k in types:
                outtype_dist[k] += 1
            if t["n_out"] <= 2:
                n_out_le2 += 1
            if "v0_p2wsh" in types:
                has_p2wsh_out += 1
                if t["n_out"] <= 2 and types.count("v0_p2wsh") == 1:
                    p2wsh_single_2out += 1
            if t["n_in"] == 2 and t["n_out"] == 3:
                shape_2in3out += 1
                if lt == 0 and allf:
                    shape_2in3out_lib += 1
                    if types.count("v0_p2wsh") == 1 and types.count("v0_p2wpkh") == 2:
                        shape_2in3out_lib_scripts += 1

    tot_in = sum(seq_vals.values())
    span = max(b["timestamp"] for b in man["blocks"]) - min(b["timestamp"] for b in man["blocks"])

    print("# Mainnet structural baseline")
    print(f"# blocks       : {len(man['blocks'])} sampled every {man['stride']} from height {man['anchor']}")
    print(f"# span         : {span/86400:.1f} days")
    print(f"# transactions : {n} non-coinbase ({sum(b['tx_count'] for b in man['blocks'])} incl. coinbase)")
    print(f"# inputs       : {tot_in}")
    print()

    print("## nSequence, per input")
    for k, v in seq_vals.most_common():
        print(f"  {k:<26} {v:>9}  {pct(v, tot_in):>6.2f}%")
    print()
    print("## nSequence, per transaction")
    print(f"  all inputs 0xffffffff      {tx_all_final:>9}  {pct(tx_all_final, n):>6.2f}%")
    print(f"  any input signals RBF      {tx_any_rbf:>9}  {pct(tx_any_rbf, n):>6.2f}%")
    print(f"  mixed sequences within tx  {mixed_seq:>9}  {pct(mixed_seq, n):>6.2f}%")
    print()

    print("## nLockTime")
    print(f"  zero                       {lock_zero:>9}  {pct(lock_zero, n):>6.2f}%")
    print(f"  height within 100 of own block (anti-fee-sniping window)")
    print(f"                             {lock_height_near:>9}  {pct(lock_height_near, n):>6.2f}%")
    print(f"  other block height         {lock_height_far:>9}  {pct(lock_height_far, n):>6.2f}%")
    print(f"  timestamp locktime         {lock_other:>9}  {pct(lock_other, n):>6.2f}%")
    print()

    print("## Metadata combinations, ranked by how common they actually are")
    rows = [
        ("all 0xfffffffd + nLockTime 0", joint_fd_lt0),
        ("all RBF-signalling + nLockTime 0", joint_rbf_lt0),
        ("all 0xffffffff + nLockTime 0  <- the library today", joint_lib),
        ("all RBF-signalling + anti-fee-sniping locktime", joint_rbf_afs),
    ]
    for lbl, v in sorted(rows, key=lambda r: -r[1]):
        print(f"  {lbl:<52} {v:>9}  {pct(v, n):>6.2f}%")
    print()

    print("## Output count")
    for k in sorted(nout_dist):
        lbl = f"{k}" if k < 20 else "20+"
        print(f"  {lbl:>3} outputs  {nout_dist[k]:>9}  {pct(nout_dist[k], n):>6.2f}%")
    print(f"  <=2 outputs  {n_out_le2:>9}  {pct(n_out_le2, n):>6.2f}%")
    print()

    print("## Input count")
    for k in sorted(nin_dist):
        lbl = f"{k}" if k < 20 else "20+"
        print(f"  {lbl:>3} inputs   {nin_dist[k]:>9}  {pct(nin_dist[k], n):>6.2f}%")
    print()

    print("## Transaction version")
    for k, v in ver_dist.most_common(6):
        print(f"  version {k:<3} {v:>9}  {pct(v, n):>6.2f}%")
    print()

    print("## Output script types (per output)")
    tot_out = sum(outtype_dist.values())
    for k, v in outtype_dist.most_common():
        print(f"  {k:<16} {v:>9}  {pct(v, tot_out):>6.2f}%")
    print()

    print("## Locating the library's transaction shape")
    print(f"  transactions with any P2WSH output        {has_p2wsh_out:>9}  {pct(has_p2wsh_out, n):>6.2f}%")
    print(f"  Kappos template (<=2 out, 1 P2WSH)       {p2wsh_single_2out:>9}  {pct(p2wsh_single_2out, n):>6.2f}%")
    print(f"  2-in 3-out (any metadata)                {shape_2in3out:>9}  {pct(shape_2in3out, n):>6.2f}%")
    print(f"  2-in 3-out + locktime 0 + all final      {shape_2in3out_lib:>9}  {pct(shape_2in3out_lib, n):>6.2f}%")
    print(f"  ...and exactly 1 P2WSH + 2 P2WPKH out    {shape_2in3out_lib_scripts:>9}  {pct(shape_2in3out_lib_scripts, n):>6.2f}%")
    print()
    print(f"  ANONYMITY SET for the library's exact structural signature:")
    print(f"    {shape_2in3out_lib_scripts} of {n} transactions = {pct(shape_2in3out_lib_scripts, n):.4f}%")
    print(f"    approx 1 in {n/shape_2in3out_lib_scripts:,.0f}" if shape_2in3out_lib_scripts else "    (none found)")


if __name__ == "__main__":
    main()
