"""Prepare (never install) current-schema Feast duty metadata for compatibility testing.

Uses the original territory-745 identity: CFC 476 / InstanceContent 40022.
Output includes original allocations, candidate allocations and unpacked overlays.
Native score/team protocol compatibility is not established by this preparation.
"""
import argparse
import json
from pathlib import Path
import struct

import feast_map as archive


def read_value(header, page, row_id, column):
    kind, offset = struct.unpack_from(">HH", header, 32 + column * 4)
    raw = archive.exd_rows(page)[row_id][1][6:]
    if kind == 0:
        fixed = struct.unpack_from(">H", header, 6)[0]
        start = fixed + struct.unpack_from(">I", raw, offset)[0]
        return raw[start:].split(b"\0", 1)[0].decode("utf-8")
    if 0x19 <= kind <= 0x20:
        return bool(raw[offset] & (1 << (kind - 0x19)))
    return struct.unpack_from({3: ">B", 4: ">h", 5: ">H", 7: ">I"}[kind], raw, offset)[0]


def edit_row(header, page, row_id, edits):
    archive.require(header[:4] == b"EXHF" and header[17] == 1, "Unsupported sheet variant")
    fixed, count = struct.unpack_from(">HH", header, 6)
    columns = list(struct.iter_unpack(">HH", header[32:32 + count * 4]))
    rows = archive.exd_rows(page)
    offset, original = rows[row_id]
    body = bytearray(original[6:6 + fixed])
    strings = bytearray(original[6 + fixed:])
    formats = {3: ">B", 4: ">h", 5: ">H", 7: ">I"}
    for column, expected_kind, value in edits:
        kind, pos = columns[column]
        archive.require(kind == expected_kind, f"Unexpected type for column {column}")
        if kind == 0:
            struct.pack_into(">I", body, pos, len(strings))
            strings.extend(value.encode("utf-8") + b"\0")
        elif 0x19 <= kind <= 0x20:
            mask = 1 << (kind - 0x19)
            body[pos] = body[pos] | mask if value else body[pos] & ~mask
        else:
            struct.pack_into(formats[kind], body, pos, value)
    payload = body + strings
    payload.extend(bytes((-len(payload) - 6) % 4))
    replacement = struct.pack(">IH", len(payload), 1) + payload
    delta = len(replacement) - len(original)
    result = bytearray(page[:offset] + replacement + page[offset + len(original):])
    index_size, data_size = struct.unpack_from(">II", result, 8)
    struct.pack_into(">I", result, 12, data_size + delta)
    for pos in range(32, 32 + index_size, 8):
        previous = struct.unpack_from(">I", result, pos + 4)[0]
        if previous > offset:
            struct.pack_into(">I", result, pos + 4, previous + delta)
    after = archive.exd_rows(result)
    archive.require(rows.keys() == after.keys(), "Row IDs changed")
    archive.require(all(raw == after[key][1] for key, (_, raw) in rows.items()
                        if key != row_id), "Unrelated row changed")
    archive.require(all(read_value(header, result, row_id, column) == expected
                        for column, _, expected in edits), "Edited field round-trip failed")
    return bytes(result)


def prepare(game, output):
    archive.require((game / "ffxivgame.ver").read_text().strip() == archive.BUILD,
                    "Unsupported client build")
    archive.require(not output.exists(), "Use a fresh output directory")
    # Explicit semantic fields from this checkout's Icarus schema. Do not copy
    # historical bytes: the old and current field offsets are different.
    cfc = [(0, 0, "l1p2"), (1, 5, 745), (2, 3, 1), (3, 5, 40022),
           (4, 0x19, True), (9, 3, 14), (12, 3, 8), (17, 3, 30),
           (43, 0, "The Feast (Crystal Tower compatibility test)"),
           (44, 0, "The Feast"), (45, 3, 6), (46, 3, 24)]
    content = [(0, 3, 5), (2, 5, 6), (4, 5, 367), (5, 5, 1),
               (8, 5, 476), (54, 7, 40001), (55, 4, -1)]
    territory = [(9, 3, 28), (10, 5, 476), (28, 0x1f, True)]
    requests = [("territorytype", "territorytype_1.exd", 745, territory),
                ("instancecontent", "instancecontent_0.exd", 40022, content)]
    requests.extend(("contentfindercondition", f"contentfindercondition_0_{lang}.exd",
                     476, cfc) for lang in ["en", "ja", "de", "fr"])
    prepared = []
    for sheet, name, row_id, edits in requests:
        header = archive.unpack_standard(archive.read_slot(game, f"exd/{sheet}.exh")[0])
        original, location = archive.read_slot(game, f"exd/{name}")
        page = archive.unpack_standard(original)
        expected = ({0: "l1p2", 9: 1, 10: 0} if sheet == "territorytype" else
                    {0: 0, 8: 0} if sheet == "instancecontent" else
                    {0: "", 1: 0, 3: 0})
        archive.require(all(read_value(header, page, row_id, column) == expected_value
                            for column, expected_value in expected.items()),
                        f"{sheet} row {row_id} is not the expected pre-restoration state")
        candidate = edit_row(header, page, row_id, edits)
        packed = archive.pack_standard(candidate, original)
        prepared.append((name, sheet, header, candidate, original, packed, {
            "resource": f"exd/{name}", "row": row_id, **location,
            "original_sha256": archive.sha(original), "candidate_sha256": archive.sha(packed),
            "schema_sha256": archive.sha(header), "edits": edits,
        }))
    # All resources must fit their existing allocations before writing any output.
    output.mkdir(parents=True)
    overlay = output / "overlay" / "exd"
    overlay.mkdir(parents=True)
    for name, sheet, header, candidate, original, packed, _ in prepared:
        (output / f"{name}.original-slot.bin").write_bytes(original)
        (output / f"{name}.candidate-slot.bin").write_bytes(packed)
        (overlay / name).write_bytes(candidate)
        (overlay / f"{sheet}.exh").write_bytes(header)
    manifest = {"build": archive.BUILD, "game": str(game.resolve()),
                "status": "prepared-only; live compatibility unverified",
                "resources": [entry[-1] for entry in prepared]}
    (output / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    print(f"Prepared {len(prepared)} resources in {output}; installation unchanged.")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--game", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    prepare(args.game, args.output)
