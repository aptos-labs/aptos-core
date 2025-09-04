'''
Today if 2 structs/enums use `CryptoHasher` derive and share the same name,
the current `CryptoHasher` implementation does not prevent hash input collision.
This can be a potential vulnerability.

The easiest way is to let velor developers ensure unique symbol names.

This script is a quick and dirty script to help find enum/structs in this repo that
use `CryptoHasher` derive and share the same name.
'''

from collections import defaultdict
from pprint import pprint
import os
import re
import subprocess

# False positives that needs to be skipped for now.
whitelisted_symbols = set([
    'TestVelorCrypto',
])

ignored_crates = set([
    'ahash',
    'velor_sdk',
    'seahash',
    'siphasher',
])

proc = subprocess.run("cargo doc --workspace --no-deps --document-private-items", shell=True)
assert proc.returncode == 0
assert os.path.exists('target/doc')

symbol_to_paths_map = defaultdict(set)
for root, dirs, files in os.walk("./target/doc"):
    path_items = root.split('/')
    assert path_items[:3] == ['.', 'target', 'doc']
    crate_name = path_items[3] if len(path_items)>=4 else None
    if crate_name in ignored_crates: continue
    relative_parent_path = '::'.join(path_items[3:])   # ./target/doc/foo/bar -> foo::bar
    for file in files:
        if not file.endswith('Hasher.html'): continue
        match = re.match(r'(?P<symbol_type>struct|enum)\.(?P<symbol_name>\w+)Hasher.html', file)
        if not match: continue
        symbol_name = match.group('symbol_name')
        relative_symbol_path = f'{relative_parent_path}::{symbol_name}'
        symbol_to_paths_map[symbol_name].add(relative_symbol_path)

print(f"The `CryptoHasher` derive are used by the following structs/enums.")
pprint(symbol_to_paths_map)
print()

if len(symbol_to_paths_map)==0:
    print(f'WARNING: no CryptoHasher derive usage found. `CryptoHasher` macro may have been updated and this check needs a rework.')
    exit(1)

reused_symbol_names = {k:v for k,v in symbol_to_paths_map.items() if len(v)>=2 and k not in whitelisted_symbols}
if reused_symbol_names:
    print(f'WARNING: the symbol name(s) below are used by 2+ structs/enums that implement `CryptoHasher`. Please ensure unique symbol names to avoid potential hash input collision across domains.')
    pprint(reused_symbol_names)
    exit(2)

print(f'No potential hash input collision across domains detected.')
