#!/usr/bin/env python3
"""Check committed campaign save archives against their integrity manifests."""

import hashlib
import json
import pathlib
import zipfile


def main():
    directory = pathlib.Path(__file__).resolve().parents[1] / 'docs/playtest-evidence/2026-10-01'
    for archive_name, manifest_name, key, prefix in [
        ('checkpoints.zip', 'manifest.json', 'sha256', 'saves/'),
        ('optional-checkpoints.zip', 'optional-manifest.json', 'hashes', ''),
    ]:
        manifest = json.loads((directory / manifest_name).read_text())
        expected = manifest[key]
        with zipfile.ZipFile(directory / archive_name) as archive:
            names = archive.namelist()
            archive_entries = {prefix + name for name in expected}
            if len(names) != len(set(names)) or set(names) != archive_entries:
                raise SystemExit(f'{archive_name}: archive/manifest entries differ')
            for name, digest in expected.items():
                if hashlib.sha256(archive.read(prefix + name)).hexdigest() != digest:
                    raise SystemExit(f'{archive_name}: checksum mismatch for {name}')
        print(f'{archive_name}: {len(expected)} save checksums verified')


if __name__ == '__main__':
    main()
