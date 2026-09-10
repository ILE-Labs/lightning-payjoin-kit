"""U2: which real outputs could actually serve as a contributor input, and at what cost?

Two independent samples of real value distributions:

  (a) FLOW  - every non-OP_RETURN output created in the sampled blocks, parsed
      from raw consensus bytes. Large, free, and a sample of newly-created UTXOs.
  (b) SPENT - the prevout values of inputs in a subsample of transactions, i.e.
      outputs someone actually chose to spend. Smaller, and the better proxy for
      what a contributor would offer, since a contributor offers a coin they are
      willing to move.

The library rejects a contributor whose UTXO cannot pay 99*feerate and still
leave 546 sat of non-dust change (src/psbt/builder.rs:11, :180-190). That is the
participation floor, and it is what this measures against.
"""
import json, sys, os, collections
sys.path.insert(0, os.path.dirname(__file__))
from parse_block import parse_block

DUST = 546
MARGINAL_VBYTES = 99


def quantiles(vals, qs=(0.01, 0.05, 0.10, 0.25, 0.50, 0.75, 0.90, 0.95, 0.99)):
    v = sorted(vals)
    return {q: v[min(len(v) - 1, int(q * len(v)))] for q in qs}


def report(name, vals, out):
    n = len(vals)
    out(f"## {name}")
    out(f"  sample size          {n:,}")
    if not n:
        return
    qd = quantiles(vals)
    out("  value quantiles (sat)")
    for q, v in qd.items():
        out(f"    p{int(q*100):<3}              {v:>15,}")
    out(f"  mean                 {sum(vals)//n:>15,}")
    out("")
    out("  share able to serve as a contributor input (value > 99*feerate + 546):")
    out("    feerate    floor(sat)      eligible        share")
    for fr in (1, 2, 5, 10, 20, 50, 100, 200, 400):
        floor = MARGINAL_VBYTES * fr + DUST
        ok = sum(1 for v in vals if v > floor)
        out(f"    {fr:>5} s/vB  {floor:>10,}  {ok:>12,}  {100.0*ok/n:>10.2f}%")
    out("")


def main():
    lines = []
    out = lines.append
    man = json.load(open("raw/manifest.json"))

    # (a) FLOW: created outputs
    created = []
    for b in man["blocks"]:
        data = open(f"raw/blocks/{b['height']}.blk", "rb").read()
        _, txs, consumed = parse_block(data)
        assert consumed == len(data)
        for t in txs:
            if t["coinbase"]:
                continue
            for val, kind in t["outputs"]:
                if kind == "op_return":
                    continue          # unspendable by construction
                created.append(val)

    # (b) SPENT: prevout values
    spent, spent_types = [], collections.Counter()
    if os.path.exists("raw/prevouts.jsonl"):
        for l in open("raw/prevouts.jsonl"):
            r = json.loads(l)
            spent.extend(r["in_vals"])
            for k in r.get("in_types", []):
                spent_types[k] += 1

    out("# U2 - real value distributions and contributor eligibility")
    out(f"# blocks: {len(man['blocks'])} sampled every {man['stride']} from height {man['anchor']}")
    out("")
    report("(a) FLOW - spendable outputs created on-chain", created, out)
    report("(b) SPENT - prevout values of inputs actually spent", spent, out)

    if spent_types:
        out("## script types of spent outputs")
        tot = sum(spent_types.values())
        for k, v in spent_types.most_common():
            out(f"  {k:<16} {v:>9}  {100.0*v/tot:>6.2f}%")
        out("")

    # Cross-check: do the two samples agree in shape?
    if created and spent:
        qc, qs = quantiles(created), quantiles(spent)
        out("## cross-check: FLOW vs SPENT quantiles (sat)")
        out("    quantile            FLOW           SPENT       ratio")
        for q in sorted(qc):
            a, b = qc[q], qs[q]
            out(f"    p{int(q*100):<3}      {a:>15,} {b:>15,}   {(b/a if a else 0):>8.2f}x")
        out("")

    # The cost in proportion, not just absolute
    out("## what the 99 vbytes costs a contributor, as a share of their coin")
    medf = quantiles(created)[0.50] if created else 0
    meds = quantiles(spent)[0.50] if spent else 0
    out(f"   FLOW median coin  = {medf:,} sat     SPENT median coin = {meds:,} sat")
    out("    feerate     cost(sat)    % of FLOW median   % of SPENT median")
    for fr in (1, 2, 5, 10, 20, 50, 100, 200, 400):
        cost = MARGINAL_VBYTES * fr
        a = 100.0 * cost / medf if medf else 0
        b = 100.0 * cost / meds if meds else 0
        out(f"    {fr:>5} s/vB  {cost:>10,}   {a:>17.3f}%  {b:>17.3f}%")
    out("")

    text = "\n".join(lines)
    print(text)
    with open("raw/utxo-dist.txt", "w") as f:
        f.write(text + "\n")


if __name__ == "__main__":
    main()
