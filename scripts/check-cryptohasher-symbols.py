'''
Today if 2 structs/enums use `CryptoHasher` derive and share the same name,
the current `CryptoHasher` implemtation does not prevent hash input collision.
This can be a potential vulnerability.

The easiest way is to let aptos developers ensure unique symbol names.

This is a quick and dirty script to help find enum/structs in this repo that
use `CryptoHasher` derive and share the same name.
Ultimately it will be done by a rust lint.

How to use:
    cargo doc --workspace --document-private-items
    python3 scripts/check-cryptohasher-symbols.py
'''

from collections import defaultdict
from pprint import pprint
import os
import re

# False positives that needs to be skipped for now.
whitelisted_symbols = set([
    'A',
    'Sea',
    'Sip',
    'TestAptosCrypto',
    'VoteProposal',
])
symbol_to_paths_map = defaultdict(set)
for root, dirs, files in os.walk("./target/doc"):
    path_pieces = root.split('/')[3:]
    if path_pieces and path_pieces[0] == 'aptos_sdk': continue
    relative_parent_path = '::'.join(path_pieces)# ./target/doc/foo/bar -> foo::bar
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
suspicious_symbols = {k:v for k,v in symbol_to_paths_map.items() if len(v)>=2 and k not in whitelisted_symbols}
if suspicious_symbols:
    print(f'WARNING: the struct/enum names below are used in 2+ places. Please rename them to avoid potential hash input collision across domains.')
    pprint(suspicious_symbols)
    exit(1)
else:
    print(f'No potential hash input collision across domains detected.')
