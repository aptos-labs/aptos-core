#!/usr/bin/env python3

# This is a script to perform mass modification of gas entries embedded in the Rust source files.
#
# It serves as a temporary solution until we come up with a more appropriate format for storing
# the gas schedule.

import os, re
from typing import Callable

root_path = os.path.dirname(os.path.abspath(__file__))

pat = re.compile("\[\s*([a-zA-Z][a-zA-Z0-9_]*)\s*:\s*([a-zA-Z][a-zA-Z0-9_]*)\s*,([^]]*),\s*([0-9][0-9_]*)\s*\,?\s*]", re.MULTILINE | re.DOTALL)

class GasEntry:
    def __init__(self, name: str, ty: str, on_chain_name: str, val: int):
        self.name = name
        self.ty = ty
        self.on_chain_name = on_chain_name
        self.val = val

def modify_file(path: str, modify_entry: Callable[[GasEntry], GasEntry]):
    with open(path, 'r') as file:
        content = file.read()

    def replace(match):
        val = int(match.group(4).replace("_", ""))
        on_chain_name: str = match.group(3).lstrip().rstrip()
        entry = GasEntry(match.group(1), match.group(2), on_chain_name, val)
        entry = modify_entry(entry)
        return "[{}: {}, {}, {}]".format(entry.name, entry.ty, entry.on_chain_name, entry.val)

    updated_content = pat.sub(replace, content)

    with open(path, 'w') as file:
        file.write(updated_content)

def modify_all_execution(modify_entry: Callable[[GasEntry], GasEntry]):
    modify_file(root_path + "/src/gas_schedule/instr.rs", modify_entry)
    modify_file(root_path + "/src/gas_schedule/move_stdlib.rs", modify_entry)
    modify_file(root_path + "/src/gas_schedule/velor_framework.rs", modify_entry)

    def wrapper(entry):
        # part of io costs
        if entry.name.startswith("common_load_"):
            return entry
        else:
            return modify_entry(entry)
    modify_file(root_path + "/src/gas_schedule/table.rs", wrapper)

def scale_all_execution(nominator: int, denominator: int):
    def scale_entry(entry):
        entry.val = (entry.val * nominator) // denominator
        return entry
    modify_all_execution(scale_entry)

def id(entry):
    return entry

# modify_all_execution(id)
scale_all_execution(1838, 10000)