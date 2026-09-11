"""Fetch a sample of mainnet blocks as raw consensus bytes.

Sampling design: blocks spaced 144 apart (about one day of blocks) walking back
from a fixed anchor height, so the sample spans roughly a month rather than
concentrating in one stretch of traffic. The anchor is pinned in the manifest so
the sample is reproducible.
"""
import json, os, sys, time, urllib.request

MIRRORS = ["https://blockstream.info/api", "https://mempool.space/api"]
OUT = "raw/blocks"
ANCHOR = int(os.environ.get("ANCHOR", "966300"))
STRIDE = int(os.environ.get("STRIDE", "144"))
COUNT = int(os.environ.get("COUNT", "31"))


def get(path, binary=False, tries=6):
    last = None
    for attempt in range(tries):
        base = MIRRORS[attempt % len(MIRRORS)]
        try:
            req = urllib.request.Request(base + path, headers={"User-Agent": "research/1.0"})
            with urllib.request.urlopen(req, timeout=120) as r:
                d = r.read()
            return d if binary else d.decode()
        except Exception as e:
            last = e
            time.sleep(2 + 3 * attempt)
    raise RuntimeError(f"failed {path}: {last}")


def main():
    os.makedirs(OUT, exist_ok=True)
    manifest = []
    heights = [ANCHOR - i * STRIDE for i in range(COUNT)]
    for n, h in enumerate(heights):
        path = f"{OUT}/{h}.blk"
        bh = get(f"/block-height/{h}").strip()
        meta = json.loads(get(f"/block/{bh}"))
        if not os.path.exists(path) or os.path.getsize(path) != meta["size"]:
            data = get(f"/block/{bh}/raw", binary=True)
            if len(data) != meta["size"]:
                raise RuntimeError(f"size mismatch at {h}: got {len(data)} want {meta['size']}")
            with open(path, "wb") as f:
                f.write(data)
        manifest.append({
            "height": h, "hash": bh, "timestamp": meta["timestamp"],
            "tx_count": meta["tx_count"], "size": meta["size"], "weight": meta["weight"],
        })
        print(f"[{n+1}/{COUNT}] {h} {bh[:16]} txs={meta['tx_count']} size={meta['size']}", flush=True)
        time.sleep(1.0)
    with open("raw/manifest.json", "w") as f:
        json.dump({"anchor": ANCHOR, "stride": STRIDE, "count": COUNT, "blocks": manifest}, f, indent=1)
    span = manifest[0]["timestamp"] - manifest[-1]["timestamp"]
    print(f"done: {sum(b['tx_count'] for b in manifest)} txs across {COUNT} blocks, span {span/86400:.1f} days")


if __name__ == "__main__":
    main()
