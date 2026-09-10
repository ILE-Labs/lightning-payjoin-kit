"""Fetch real input (prevout) values for a subsample of transactions.

Raw blocks carry only outpoints, not the values being spent. The explorer's tx
endpoint resolves prevouts, so this gives a genuine sample of *what a UTXO that
actually gets spent looks like* - which is the population a contributor's input
would be drawn from, and a better proxy for that than the raw UTXO set stock.

Paged 25 transactions at a time, so this is deliberately a subsample.
"""
import json, os, time, urllib.request

MIRRORS = ["https://blockstream.info/api", "https://mempool.space/api"]
PAGES_PER_BLOCK = int(os.environ.get("PAGES_PER_BLOCK", "12"))  # 12 * 25 = 300 txs
OUT = "raw/prevouts.jsonl"


class NoSuchPage(Exception):
    pass


def get(path, tries=6):
    """A 404 on a tx page means the block has no more pages, not a failure.
    Mirrors disagree on how they signal the end of pagination, so treat a 404
    seen on every mirror as end-of-block rather than aborting the whole run."""
    last = None
    saw_404 = 0
    for a in range(tries):
        try:
            req = urllib.request.Request(MIRRORS[a % len(MIRRORS)] + path,
                                         headers={"User-Agent": "research/1.0"})
            with urllib.request.urlopen(req, timeout=120) as r:
                return r.read().decode()
        except urllib.error.HTTPError as e:
            last = e
            if e.code == 404:
                saw_404 += 1
                if saw_404 >= len(MIRRORS):
                    raise NoSuchPage(path)
            time.sleep(1 + 2 * a)
        except Exception as e:
            last = e
            time.sleep(2 + 3 * a)
    raise RuntimeError(f"failed {path}: {last}")


def main():
    man = json.load(open("raw/manifest.json"))
    seen = 0
    with open(OUT, "w") as out:
        for b in man["blocks"]:
            for p in range(PAGES_PER_BLOCK):
                # Skip index 0 page start for coinbase handling; coinbase filtered below.
                try:
                    txs = json.loads(get(f"/block/{b['hash']}/txs/{p*25}"))
                except NoSuchPage:
                    break
                if not txs:
                    break
                for t in txs:
                    if any(v.get("is_coinbase") for v in t["vin"]):
                        continue
                    rec = {
                        "h": b["height"],
                        "nin": len(t["vin"]),
                        "nout": len(t["vout"]),
                        "in_vals": [v["prevout"]["value"] for v in t["vin"] if v.get("prevout")],
                        "in_types": [v["prevout"]["scriptpubkey_type"] for v in t["vin"] if v.get("prevout")],
                        "out_vals": [o["value"] for o in t["vout"]],
                        "fee": t.get("fee"),
                        "weight": t.get("weight"),
                    }
                    out.write(json.dumps(rec) + "\n")
                    seen += 1
                time.sleep(0.35)
            print(f"block {b['height']}: cumulative {seen} txs", flush=True)
    print(f"done: {seen} transactions with resolved prevout values")


if __name__ == "__main__":
    main()
