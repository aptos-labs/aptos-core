'''
Today if 2 structs/enums use `CryptoHasher` derive and share the same name,
the current `CryptoHasher` implemtation does not prevent hash input collision.
This can be a potential vulnerability.

The easiest way is to let aptos developers ensure unique symbol names.

This is a quick and dirty script to help find enum/structs in this repo that
use `CryptoHasher` derive and share the same name.
Ultimately it will be done by a rust lint.

How to use:
    python3 scripts/check-cryptohashable-symbols.py
'''

from collections import defaultdict
from pathlib import Path
from pprint import pprint
import os
import re

print(f'Searching for structs/enums that implements CryptoHasher...')
ST_INIT = 0
ST_DERIVE_FOUND = 1

def find_cryptohasher_implementors_in_path(path):
    code = path.read_text()
    lines = code.split('\n')
    n = len(lines)
    st = ST_INIT
    ret = set()
    for i in range(n):
        line_number = i+1
        if st==ST_INIT:
            match = re.match(r'#\[derive\(.*(CryptoHasher).*\)', lines[i])
            if match:
                st = ST_DERIVE_FOUND
        elif st==ST_DERIVE_FOUND:
            match = re.match(r'.*(struct|enum)\s+(\w+).*', lines[i])
            if match:
                symbol_name = match.group(2)
                ret.add((symbol_name,line_number))
                st = ST_INIT
        else:
            assert False
    return ret

symbol_map = defaultdict(set)
for root, dirs, files in os.walk("."):
    if root.startswith('./target/'): continue
    for file in files:
        if file.endswith(".rs"):
            file_path = Path(f'{root}/{file}')
            for symbol,line_number in find_cryptohasher_implementors_in_path(file_path):
                symbol_map[symbol].add(f'{file_path}:{line_number}')

print(f"The `CryptoHasher` derive are used by the following structs/enums.")
pprint(symbol_map)
print()
suspicious_symbol_map = {k:v for k,v in symbol_map.items() if len(v)>=2}
if suspicious_symbol_map:
    print(f'WARNING: the struct/enum names below are used in 2+ places. ')
    pprint(suspicious_symbol_map)
