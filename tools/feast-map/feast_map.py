"""Restore territory 745 for exploration on Kawari's 7.51h2 Windows client.

Python 3.9+, standard library only. prepare is read-only with respect to the game.
install/restore touch one existing SqPack allocation, with a verified backup.
Format references: redstrate/physis (exh.rs, exd.rs, sqpack/{index,data}.rs).
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import struct
import subprocess
import zlib

BUILD = "2026.06.18.0000.0000"
TERRITORY = 745
BG = "ffxiv/pvp_xx/pvp/l1p2/level/l1p2"
HEADER_PATH = "exd/territorytype.exh"
PAGE_PATH = "exd/territorytype_1.exd"
INDEX_PATH = "sqpack/ffxiv/0a0000.win32.index2"


def require(condition, message):
    if not condition:
        raise ValueError(message)


def sha(data):
    return hashlib.sha256(data).hexdigest()


def index_entries(data):
    require(data[:6] == b"SqPack", "Not a SqPack index")
    header = struct.unpack_from("<I", data, 12)[0]
    offset, size = struct.unpack_from("<II", data, header + 8)
    return dict(struct.iter_unpack("<II", data[offset:offset + size]))


def read_slot(game, name):
    index = (game / INDEX_PATH).read_bytes()
    entries = index_entries(index)
    entry = entries[zlib.crc32(name.encode()) ^ 0xFFFFFFFF]
    require(not entry & 1, "Synonym index entries are unsupported")
    dat_id, offset = (entry & 14) >> 1, (entry & ~15) * 8
    relative = f"sqpack/ffxiv/0a0000.win32.dat{dat_id}"
    with (game / relative).open("rb") as file:
        file.seek(offset)
        header_size, kind, _, reserved, _, _ = struct.unpack("<6I", file.read(24))
        require(kind == 2, "Expected a standard SqPack resource")
        length = header_size + reserved * 128
        following = [(v & ~15) * 8 for v in entries.values()
                     if not v & 1 and (v & 14) >> 1 == dat_id and (v & ~15) * 8 > offset]
        require(offset + length <= min(following, default=(game / relative).stat().st_size),
                "Allocation overlaps another resource")
        file.seek(offset)
        raw = file.read(length)
    require(len(raw) == length, "Truncated resource allocation")
    return raw, {"dat": relative, "offset": offset, "length": length,
                 "index_sha256": sha(index)}


def unpack_standard(raw):
    header, kind, size, _, _, count = struct.unpack_from("<6I", raw)
    require(kind == 2 and 24 + count * 8 <= header, "Invalid standard resource header")
    result = bytearray()
    for i in range(count):
        relative = struct.unpack_from("<I", raw, 24 + i * 8)[0]
        pos = header + relative
        block_header, _, compressed, uncompressed = struct.unpack_from("<4I", raw, pos)
        require(block_header == 16, "Unexpected compression block header")
        length = uncompressed if compressed == 32000 else compressed
        payload = raw[pos + 16:pos + 16 + length]
        block = payload if compressed == 32000 else zlib.decompress(payload, -15)
        require(len(block) == uncompressed, "Decompressed block size mismatch")
        result.extend(block)
    require(len(result) == size, "Decompressed resource size mismatch")
    return bytes(result)


def pack_standard(data, original):
    header_size, _, _, reserved, _, _ = struct.unpack_from("<6I", original)
    blocks, table = bytearray(), bytearray()
    for start in range(0, len(data), 16000):
        part = data[start:start + 16000]
        compressor = zlib.compressobj(9, zlib.DEFLATED, -15)
        compressed = compressor.compress(part) + compressor.flush()
        use_compression = len(compressed) < len(part)
        payload = compressed if use_compression else part
        marker = len(compressed) if use_compression else 32000
        block_size = (16 + len(payload) + 127) & ~127
        table.extend(struct.pack("<IHH", len(blocks), block_size, len(part)))
        blocks.extend(struct.pack("<4I", 16, 0, marker, len(part)))
        blocks.extend(payload)
        blocks.extend(bytes(block_size - 16 - len(payload)))
    require(24 + len(table) <= header_size, "New block table exceeds existing allocation")
    require(len(blocks) <= reserved * 128, "Patched page does not fit; no game files changed")
    raw = bytearray(len(original))
    struct.pack_into("<6I", raw, 0, header_size, 2, len(data), reserved,
                     len(blocks) // 128, len(table) // 8)
    raw[24:24 + len(table)] = table
    raw[header_size:header_size + len(blocks)] = blocks
    require(unpack_standard(raw) == data, "SqPack round-trip failed")
    return bytes(raw)


def exd_rows(data):
    require(data[:4] == b"EXDF", "Not an EXD page")
    index_size, data_size = struct.unpack_from(">II", data, 8)
    require(len(data) == 32 + index_size + data_size, "Unexpected EXD page length")
    rows = {}
    for row_id, offset in struct.iter_unpack(">II", data[32:32 + index_size]):
        size, count = struct.unpack_from(">IH", data, offset)
        require(count == 1 and offset + 6 + size <= len(data), "Invalid EXD row")
        rows[row_id] = (offset, data[offset:offset + 6 + size])
    return rows


def restore_row(exh, page):
    require(exh[:4] == b"EXHF" and exh[17] == 1, "Expected a single-row Excel sheet")
    fixed_size, count = struct.unpack_from(">HH", exh, 6)
    require(count == 46, "TerritoryType schema changed; update this tool for the new client")
    columns = list(struct.iter_unpack(">HH", exh[32:32 + count * 4]))
    before = exd_rows(page)
    offset, old = before[TERRITORY]
    body = bytearray(old[6:6 + fixed_size])
    require(len(body) == fixed_size, "Truncated territory row")
    bg_type, bg_offset = columns[1]
    require(bg_type == 0, "Bg is not a string column")
    string_offset = struct.unpack_from(">I", body, bg_offset)[0]
    old_bg = old[6 + fixed_size + string_offset:].split(b"\0", 1)[0]
    require(not old_bg, "Territory 745 is already populated; refusing to overwrite it")
    name_offset = struct.unpack_from(">I", body, columns[0][1])[0]
    require(not old[6 + fixed_size + name_offset:].split(b"\0", 1)[0],
            "Territory 745 has been repurposed; refusing to overwrite it")
    strings = bytearray()

    def field(column, expected, value):
        kind, pos = columns[column]
        require(kind == expected, f"Unexpected schema type in column {column}")
        if kind == 0:
            struct.pack_into(">I", body, pos, len(strings))
            strings.extend(value.encode() + b"\0")
        elif 0x19 <= kind <= 0x20:
            bit = 1 << (kind - 0x19)
            body[pos] = (body[pos] | bit) if value else (body[pos] & ~bit)
        else:
            struct.pack_into({3: ">B", 5: ">H", 6: ">i", 2: ">b"}[kind], body, pos, value)

    # Column meanings from the TerritoryType schema used by this checkout's Icarus.
    field(0, 0, "l1p2")
    field(1, 0, BG)
    for column, value in [(2, 1), (7, 6), (8, 0), (9, 1), (12, 90)]:
        field(column, 3, value)
    for column, value in [(3, 26), (4, 501), (5, 2383), (6, 440), (10, 0), (19, 1001)]:
        field(column, 5, value)
    for column, value in [(20, -1), (21, -1), (25, 1100)]:
        field(column, 6, value)
    field(27, 2, -1)
    field(15, 0x1B, True)  # PCSearch
    field(35, 0x19, True)  # IsInUse
    payload = body + strings
    payload.extend(bytes((-len(payload) - 6) % 4))
    replacement = struct.pack(">IH", len(payload), 1) + payload
    delta = len(replacement) - len(old)
    result = bytearray(page[:offset] + replacement + page[offset + len(old):])
    index_size, data_size = struct.unpack_from(">II", result, 8)
    struct.pack_into(">I", result, 12, data_size + delta)
    for pos in range(32, 32 + index_size, 8):
        old_offset = struct.unpack_from(">I", result, pos + 4)[0]
        if old_offset > offset:
            struct.pack_into(">I", result, pos + 4, old_offset + delta)
    after = exd_rows(result)
    require(before.keys() == after.keys(), "Row IDs changed")
    require(all(old_row == after[row_id][1] for row_id, (_, old_row) in before.items()
                if row_id != TERRITORY), "An unrelated territory row changed")
    return bytes(result)


def prepare(game, output):
    require((game / "ffxivgame.ver").read_text().strip() == BUILD, "Unsupported client build")
    require(not (output / "patch.json").exists(), "Use a new output folder; preserve existing backups")
    original, location = read_slot(game, PAGE_PATH)
    exh = unpack_standard(read_slot(game, HEADER_PATH)[0])
    old_page = unpack_standard(original)
    new_page = restore_row(exh, old_page)
    patched = pack_standard(new_page, original)
    output.mkdir(parents=True, exist_ok=True)
    (output / "original-slot.bin").write_bytes(original)
    (output / "patched-slot.bin").write_bytes(patched)
    overlay = output / "overlay/exd"
    overlay.mkdir(parents=True, exist_ok=True)
    (overlay / "territorytype_1.exd").write_bytes(new_page)
    (overlay / "territorytype.exh").write_bytes(exh)
    info = {"build": BUILD, "game": str(game.resolve()), "resource": PAGE_PATH,
            **location, "original_sha256": sha(original), "patched_sha256": sha(patched),
            "exh_sha256": sha(exh), "territory": TERRITORY, "bg": BG}
    (output / "patch.json").write_text(json.dumps(info, indent=2) + "\n")
    print(f"Prepared territory {TERRITORY}; only that EXD row changed. Backup: {output}")


def ensure_stopped():
    if os.name == "nt":
        result = subprocess.run(["tasklist", "/FO", "CSV", "/NH"],
                                check=True, capture_output=True, text=True)
        names = [line.split(',')[0].strip('"').lower() for line in result.stdout.splitlines()]
        running = [n for n in names if n.startswith(("ffxiv", "kawari-", "astra"))]
        require(not running, "Close game, launchers and Kawari before patching: " + ", ".join(running))


def apply(output, restore=False):
    info = json.loads((output / "patch.json").read_text())
    game = Path(info["game"])
    require(info["build"] == BUILD and (game / "ffxivgame.ver").read_text().strip() == BUILD,
            "Client updated; refusing to apply a stale backup")
    require(info["resource"] == PAGE_PATH, "Unexpected target resource")
    original = (output / "original-slot.bin").read_bytes()
    patched = (output / "patched-slot.bin").read_bytes()
    require(sha(original) == info["original_sha256"] and sha(patched) == info["patched_sha256"],
            "Backup or patch checksum mismatch")
    require(len(original) == len(patched) == info["length"], "Patch allocation size mismatch")
    current, location = read_slot(game, PAGE_PATH)
    require(all(info[k] == location[k] for k in location), "Archive layout/index changed")
    require(sha(unpack_standard(read_slot(game, HEADER_PATH)[0])) == info["exh_sha256"],
            "TerritoryType schema changed")
    desired, expected = (original, patched) if restore else (patched, original)
    if current == desired:
        print("Already restored." if restore else "Already installed.")
        return
    require(current == expected, "Target data was modified by something else; refusing to overwrite it")
    ensure_stopped()
    target = game / location["dat"]
    with target.open("r+b") as file:
        file.seek(location["offset"])
        require(file.read(location["length"]) == expected, "Target changed before write")
        try:
            file.seek(location["offset"])
            require(file.write(desired) == len(desired), "Short write")
            file.flush()
            os.fsync(file.fileno())
            file.seek(location["offset"])
            require(file.read(len(desired)) == desired, "Read-back verification failed")
        except Exception:
            file.seek(location["offset"])
            file.write(expected)
            file.flush()
            os.fsync(file.fileno())
            raise
    print("Restored original territory data." if restore else "Installed. Restart client and server, then use !feast.")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["prepare", "install", "restore"])
    parser.add_argument("--output", type=Path, required=True, help="Patch and backup directory")
    parser.add_argument("--game", type=Path, help="FFXIV game directory (prepare only)")
    args = parser.parse_args()
    if args.action == "prepare":
        if args.game is None:
            parser.error("--game is required")
        prepare(args.game, args.output)
    else:
        apply(args.output, restore=args.action == "restore")


if __name__ == "__main__":
    main()
