"""Exercise installation/rollback on synthetic archives, never an installed game."""
import json
from pathlib import Path
import struct
import tempfile
import unittest
from unittest.mock import patch
import zlib

import feast_map as fm


def slot(payload, reserved=16):
    blank = bytearray(128 + reserved * 128)
    struct.pack_into("<6I", blank, 0, 128, 2, 0, reserved, 0, 0)
    return fm.pack_standard(payload, blank)


class InstallationTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.game = self.root / "game"
        self.output = self.root / "backup"
        self.output.mkdir()
        pack = self.game / "sqpack/ffxiv"
        pack.mkdir(parents=True)
        (self.game / "ffxivgame.ver").write_text(fm.BUILD)
        self.original = slot(b"original territory data" * 250)
        self.patched = slot(b"restored territory data" * 250)
        self.exh = b"synthetic schema for installation tests"
        data = bytearray(b"\xA5" * 8192)
        data[2048:2304] = slot(self.exh, reserved=1)
        data[4096:4096 + len(self.original)] = self.original
        self.dat = pack / "0a0000.win32.dat0"
        self.dat.write_bytes(data)
        self.initial_file = bytes(data)
        index = bytearray(2064)
        index[:6] = b"SqPack"
        struct.pack_into("<I", index, 12, 1024)
        struct.pack_into("<II", index, 1032, 2048, 16)
        for i, (name, offset) in enumerate([(fm.HEADER_PATH, 2048), (fm.PAGE_PATH, 4096)]):
            struct.pack_into("<II", index, 2048 + i * 8,
                             zlib.crc32(name.encode()) ^ 0xFFFFFFFF, offset // 8)
        (self.game / fm.INDEX_PATH).write_bytes(index)
        _, location = fm.read_slot(self.game, fm.PAGE_PATH)
        info = {"game": str(self.game), "build": fm.BUILD, "resource": fm.PAGE_PATH,
                **location, "original_sha256": fm.sha(self.original),
                "patched_sha256": fm.sha(self.patched), "exh_sha256": fm.sha(self.exh)}
        (self.output / "patch.json").write_text(json.dumps(info))
        (self.output / "original-slot.bin").write_bytes(self.original)
        (self.output / "patched-slot.bin").write_bytes(self.patched)

    @patch.object(fm, "ensure_stopped")
    def test_install_is_idempotent_and_restore_is_byte_exact(self, stopped):
        fm.apply(self.output)
        fm.apply(self.output)
        data = self.dat.read_bytes()
        self.assertEqual(data[:4096], self.initial_file[:4096])
        self.assertEqual(data[4096 + len(self.patched):], self.initial_file[4096 + len(self.patched):])
        self.assertEqual(fm.read_slot(self.game, fm.PAGE_PATH)[0], self.patched)
        fm.apply(self.output, restore=True)
        fm.apply(self.output, restore=True)
        self.assertEqual(self.dat.read_bytes(), self.initial_file)

    def test_client_update_rejected_without_writes(self):
        (self.game / "ffxivgame.ver").write_text("new build")
        with self.assertRaisesRegex(ValueError, "Client updated"):
            fm.apply(self.output)
        self.assertEqual(self.dat.read_bytes(), self.initial_file)

    def test_other_modification_is_not_overwritten(self):
        data = bytearray(self.initial_file)
        data[4200] ^= 1
        self.dat.write_bytes(data)
        with self.assertRaisesRegex(ValueError, "modified by something else"):
            fm.apply(self.output, restore=True)
        self.assertEqual(self.dat.read_bytes(), data)

    def test_corrupted_backup_rejected(self):
        (self.output / "original-slot.bin").write_bytes(b"damaged")
        with self.assertRaisesRegex(ValueError, "checksum mismatch"):
            fm.apply(self.output)
        self.assertEqual(self.dat.read_bytes(), self.initial_file)

    def test_index_update_rejected(self):
        index = self.game / fm.INDEX_PATH
        data = bytearray(index.read_bytes())
        data[100] ^= 1
        index.write_bytes(data)
        with self.assertRaisesRegex(ValueError, "layout/index changed"):
            fm.apply(self.output)
        self.assertEqual(self.dat.read_bytes(), self.initial_file)


class CompressionTests(unittest.TestCase):
    def test_compressed_and_raw_blocks_round_trip(self):
        import random
        data = b"a" * 16000 + random.Random(42).randbytes(16000)
        self.assertEqual(fm.unpack_standard(slot(data, reserved=512)), data)

    def test_oversized_replacement_rejected(self):
        import random
        data = random.Random(17).randbytes(16000)
        with self.assertRaisesRegex(ValueError, "does not fit"):
            slot(data, reserved=1)


if __name__ == "__main__":
    unittest.main()
