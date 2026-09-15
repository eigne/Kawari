import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import feast_map as archive
import install_feast_duty as installer


class DutyInstallTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        (self.root / "ffxivgame.ver").write_text(archive.BUILD)
        names = ["territorytype_1.exd", "instancecontent_0.exd"]
        names += [f"contentfindercondition_0_{lang}.exd" for lang in ["en", "ja", "de", "fr"]]
        entries = []
        for i, name in enumerate(names):
            original, candidate = bytes([i]) * 16, bytes([i + 10]) * 16
            (self.root / f"{name}.original-slot.bin").write_bytes(original)
            (self.root / f"{name}.candidate-slot.bin").write_bytes(candidate)
            (self.root / f"{name}.dat").write_bytes(original)
            entries.append({"resource": f"exd/{name}", "dat": f"{name}.dat", "offset": 0,
                            "length": 16, "index_sha256": "test",
                            "schema_sha256": archive.sha(b"header"),
                            "original_sha256": archive.sha(original),
                            "candidate_sha256": archive.sha(candidate)})
        self.entries = entries
        (self.root / "manifest.json").write_text(json.dumps({
            "game": str(self.root), "build": archive.BUILD, "resources": entries}))

        def read_slot(game, resource):
            if resource.endswith(".exh"):
                return b"header", {}
            entry = next(x for x in entries if x["resource"] == resource)
            return (game / entry["dat"]).read_bytes(), {
                k: entry[k] for k in ["dat", "offset", "length", "index_sha256"]}

        for mock in [patch.object(archive, "ensure_stopped"),
                     patch.object(archive, "unpack_standard", side_effect=lambda x: x),
                     patch.object(archive, "read_slot", side_effect=read_slot)]:
            mock.start()
            self.addCleanup(mock.stop)

    def test_install_restore_and_repeat(self):
        installer.apply(self.root)
        installer.apply(self.root)
        for entry in self.entries:
            self.assertEqual(archive.sha((self.root / entry["dat"]).read_bytes()),
                             entry["candidate_sha256"])
        installer.apply(self.root, restore=True)
        for entry in self.entries:
            self.assertEqual(archive.sha((self.root / entry["dat"]).read_bytes()),
                             entry["original_sha256"])

    def test_late_preflight_failure_leaves_earlier_resources_unchanged(self):
        (self.root / self.entries[-1]["dat"]).write_bytes(b"unrelated change")
        with self.assertRaises(ValueError):
            installer.apply(self.root)
        for entry in self.entries[:-1]:
            self.assertEqual(archive.sha((self.root / entry["dat"]).read_bytes()),
                             entry["original_sha256"])

    def test_write_failure_rolls_back_all_touched_resources(self):
        real_sync = installer.os.fsync
        calls = 0

        def fail_second(fd):
            nonlocal calls
            calls += 1
            if calls == 2:
                raise OSError("simulated disk error")
            return real_sync(fd)

        with patch.object(installer.os, "fsync", side_effect=fail_second):
            with self.assertRaises(OSError):
                installer.apply(self.root)
        for entry in self.entries:
            self.assertEqual(archive.sha((self.root / entry["dat"]).read_bytes()),
                             entry["original_sha256"])


if __name__ == "__main__":
    unittest.main()
