"""Match recovered ULD filename hashes against strings in an old client EXE.

Read-only: never executes the client or copies proprietary resources.
Usage: python resolve-ui-names.py OLD_EXE UNIDENTIFIED_DIRECTORY
"""
import argparse
import json
from pathlib import Path
import re
import zlib

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('executable', type=Path)
parser.add_argument('unidentified', type=Path)
args = parser.parse_args()
unknown = {p.stem.removeprefix('uld_').lower()
           for p in args.unidentified.glob('uld_*.uld')}
strings = set(re.findall(rb'[ -~]{4,160}', args.executable.read_bytes()))
matches = {}
for text in strings:
    for word in re.findall(r'[A-Za-z_][A-Za-z0-9_]{2,90}', text.decode('ascii')):
        for suffix in ['.uld', '0.uld', '1.uld', '2.uld', '3.uld']:
            name = (word + suffix).lower()
            crc = f'{(~zlib.crc32(name.encode("ascii"))) & 0xffffffff:08x}'
            if crc in unknown:
                matches.setdefault(crc, set()).add(name)
print(json.dumps({h: sorted(names) for h, names in sorted(matches.items())}, indent=2))
