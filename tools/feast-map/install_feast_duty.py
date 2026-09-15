"""Install or restore a prepared Feast duty candidate, with all-resource preflight.

Both client and server must be closed. Restore returns to the exploration patch.
"""
import argparse
import json
import os
from pathlib import Path

import feast_map as archive


def apply(output, restore=False):
    info = json.loads((output / "manifest.json").read_text(encoding="utf-8"))
    game = Path(info["game"])
    archive.require(info["build"] == archive.BUILD
                    and (game / "ffxivgame.ver").read_text().strip() == archive.BUILD,
                    "Unsupported or updated client")
    expected_resources = {"exd/territorytype_1.exd", "exd/instancecontent_0.exd"}
    expected_resources.update(f"exd/contentfindercondition_0_{lang}.exd"
                              for lang in ["en", "ja", "de", "fr"])
    entries = info["resources"]
    archive.require(len(entries) == 6 and {x["resource"] for x in entries} == expected_resources,
                    "Unexpected resource set")
    archive.ensure_stopped()
    pending = []
    for entry in entries:
        resource = entry["resource"]
        name = Path(resource).name
        sheet = name.split("_")[0]
        header = archive.unpack_standard(archive.read_slot(game, f"exd/{sheet}.exh")[0])
        archive.require(archive.sha(header) == entry["schema_sha256"], "Schema changed")
        original = (output / f"{name}.original-slot.bin").read_bytes()
        candidate = (output / f"{name}.candidate-slot.bin").read_bytes()
        archive.require(archive.sha(original) == entry["original_sha256"]
                        and archive.sha(candidate) == entry["candidate_sha256"], "Checksum mismatch")
        current, location = archive.read_slot(game, resource)
        archive.require(all(entry[k] == v for k, v in location.items()), "Archive layout changed")
        archive.require(len(original) == len(candidate) == location["length"], "Length mismatch")
        desired, expected = (original, candidate) if restore else (candidate, original)
        archive.require(current in (desired, expected), "Resource changed outside this patch")
        if current != desired:
            pending.append((game / location["dat"], location["offset"], current, desired))
    changed = []
    try:
        for path, offset, current, desired in pending:
            with path.open("r+b") as file:
                file.seek(offset)
                archive.require(file.read(len(current)) == current, "Resource changed before write")
                changed.append((path, offset, current))
                file.seek(offset)
                archive.require(file.write(desired) == len(desired), "Short write")
                file.flush()
                os.fsync(file.fileno())
                file.seek(offset)
                archive.require(file.read(len(desired)) == desired, "Read-back mismatch")
    except Exception:
        # Roll back every allocation touched in this invocation, including a
        # partially written current allocation. Original backups remain on disk.
        for path, offset, original in reversed(changed):
            with path.open("r+b") as file:
                file.seek(offset)
                archive.require(file.write(original) == len(original), "Rollback short write")
                file.flush()
                os.fsync(file.fileno())
                file.seek(offset)
                archive.require(file.read(len(original)) == original, "Rollback verification failed")
        raise
    print(f"{'Restored exploration' if restore else 'Installed duty candidate'}: {len(pending)} allocations updated.")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--restore", action="store_true")
    args = parser.parse_args()
    apply(args.output, args.restore)
