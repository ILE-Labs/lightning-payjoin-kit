"""Minimal Bitcoin block parser.

Parses raw consensus-serialised blocks and extracts only the fields the
structural-indistinguishability question needs: input and output counts,
nSequence per input, nLockTime, transaction version, and output script types.

Self-checking: the caller verifies that the parsed transaction count matches the
block header's count and that the parser consumed exactly the block's byte length.
A parser that silently desynchronises would fail both checks.
"""
import struct


class Reader:
    def __init__(self, b):
        self.b = b
        self.i = 0

    def take(self, n):
        if self.i + n > len(self.b):
            raise ValueError(f"read past end: want {n} at {self.i} of {len(self.b)}")
        v = self.b[self.i:self.i + n]
        self.i += n
        return v

    def u8(self):
        return self.take(1)[0]

    def u16(self):
        return struct.unpack("<H", self.take(2))[0]

    def u32(self):
        return struct.unpack("<I", self.take(4))[0]

    def u64(self):
        return struct.unpack("<Q", self.take(8))[0]

    def i32(self):
        return struct.unpack("<i", self.take(4))[0]

    def varint(self):
        n = self.u8()
        if n < 0xFD:
            return n
        if n == 0xFD:
            return self.u16()
        if n == 0xFE:
            return self.u32()
        return self.u64()


def classify_spk(spk: bytes) -> str:
    """Standard output script types, by exact shape."""
    n = len(spk)
    if n == 22 and spk[0] == 0x00 and spk[1] == 0x14:
        return "v0_p2wpkh"
    if n == 34 and spk[0] == 0x00 and spk[1] == 0x20:
        return "v0_p2wsh"
    if n == 34 and spk[0] == 0x51 and spk[1] == 0x20:
        return "v1_p2tr"
    if n == 25 and spk[0:3] == b"\x76\xa9\x14" and spk[23:25] == b"\x88\xac":
        return "p2pkh"
    if n == 23 and spk[0] == 0xa9 and spk[1] == 0x14 and spk[22] == 0x87:
        return "p2sh"
    if n >= 1 and spk[0] == 0x6a:
        return "op_return"
    if n in (35, 67) and spk[-1] == 0xac:
        return "p2pk"
    # Other witness versions: OP_1..OP_16 followed by a 2-40 byte push.
    if 4 <= n <= 42 and (spk[0] == 0x51 or 0x52 <= spk[0] <= 0x60):
        if spk[1] == n - 2:
            return "witness_other"
    return "nonstandard"


def parse_tx(r: Reader):
    start = r.i
    version = r.i32()
    marker_pos = r.i
    n_in = r.varint()
    segwit = False
    if n_in == 0:
        flag = r.u8()
        if flag != 0x01:
            raise ValueError(f"bad segwit flag {flag}")
        segwit = True
        n_in = r.varint()
    else:
        r.i = marker_pos
        n_in = r.varint()

    seqs = []
    is_coinbase = False
    for k in range(n_in):
        prev_hash = r.take(32)
        prev_idx = r.u32()
        if k == 0 and prev_hash == b"\x00" * 32 and prev_idx == 0xFFFFFFFF:
            is_coinbase = True
        script_len = r.varint()
        script_sig = r.take(script_len)
        seqs.append((r.u32(), len(script_sig)))

    n_out = r.varint()
    outs = []
    for _ in range(n_out):
        value = r.u64()
        spk_len = r.varint()
        spk = r.take(spk_len)
        outs.append((value, classify_spk(spk)))

    n_witness_items = 0
    if segwit:
        for _ in range(n_in):
            items = r.varint()
            n_witness_items += items
            for _ in range(items):
                ln = r.varint()
                r.take(ln)

    locktime = r.u32()
    return {
        "version": version,
        "segwit": segwit,
        "coinbase": is_coinbase,
        "n_in": n_in,
        "n_out": n_out,
        "sequences": [s for s, _ in seqs],
        "scriptsig_lens": [l for _, l in seqs],
        "outputs": outs,
        "locktime": locktime,
        "size": r.i - start,
    }


def parse_block(data: bytes):
    r = Reader(data)
    header = {
        "version": r.i32(),
        "prev": r.take(32),
        "merkle": r.take(32),
        "time": r.u32(),
        "bits": r.u32(),
        "nonce": r.u32(),
    }
    n_tx = r.varint()
    txs = [parse_tx(r) for _ in range(n_tx)]
    return header, txs, r.i
