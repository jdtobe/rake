#!/usr/bin/env python3
"""Stamp only the root package and its lock entry, validating before mutation."""
from pathlib import Path
import re
import sys
import tomllib


def stamp(tag):
    if not re.fullmatch(r'v[1-9][0-9]\.(?:[1-9]|1[0-2])\.(?:0|[1-9][0-9]*)', tag):
        raise ValueError('invalid calendar tag: expected vYY.M.N without leading zeroes')
    version = tag[1:]
    manifest_path, lock_path = Path('Cargo.toml'), Path('Cargo.lock')
    manifest, lock = manifest_path.read_text(), lock_path.read_text()
    package = tomllib.loads(manifest)['package']
    parsed_lock = tomllib.loads(lock)
    roots = [entry for entry in parsed_lock.get('package', [])
             if entry['name'] == package['name'] and 'source' not in entry]
    if len(roots) != 1 or roots[0]['version'] != package['version']:
        raise ValueError('missing or mismatched root package in Cargo.lock')
    section = re.search(r'(?ms)^\[package\][ \t]*\n.*?(?=^\[|\Z)', manifest)
    if section is None:
        raise ValueError('missing package section')

    def replace_version(text):
        updated, count = re.subn(r'(?m)^(version\s*=\s*)"[^"]+"',
                                 lambda match: match[1] + f'"{version}"', text, count=1)
        if count != 1:
            raise ValueError('missing version field')
        return updated

    updated_manifest = manifest[:section.start()] + replace_version(section[0]) + manifest[section.end():]
    blocks = re.split(r'(?m)(?=^\[\[package\]\][ \t]*$)', lock)
    for index, block in enumerate(blocks):
        entries = tomllib.loads(block).get('package', [])
        if entries and entries[0]['name'] == package['name'] and 'source' not in entries[0]:
            blocks[index] = replace_version(block)
    updated_lock = ''.join(blocks)
    # Parse both prepared results before touching either file.
    tomllib.loads(updated_manifest)
    tomllib.loads(updated_lock)
    manifest_path.write_text(updated_manifest)
    lock_path.write_text(updated_lock)


if __name__ == '__main__':
    try:
        if len(sys.argv) != 2:
            raise ValueError('usage: stamp-version.py vYY.M.N')
        stamp(sys.argv[1])
    except (ValueError, KeyError, OSError) as error:
        sys.exit(str(error))
