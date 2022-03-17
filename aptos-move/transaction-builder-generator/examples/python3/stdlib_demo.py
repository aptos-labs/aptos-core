# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

# pyre-strict

import aptos_types as aptos
import serde_types as st
import aptos_framework as stdlib


def demo_p2p_script() -> None:
    token = aptos.TypeTag__Struct(
        value=aptos.StructTag(
            address=aptos.AccountAddress.from_bytes(b"\x00" * 15 + b"\x01"),
            module=aptos.Identifier("XDX"),
            name=aptos.Identifier("XDX"),
            type_params=[],
        )
    )
    payee = aptos.AccountAddress.from_bytes(b"\x22" * 16)
    amount = st.uint64(1_234_567)
    script = stdlib.encode_peer_to_peer_with_metadata_script(token, payee, amount, b"", b"")

    call = stdlib.decode_script(script)
    assert isinstance(call, stdlib.ScriptCall__PeerToPeerWithMetadata)
    assert call.amount == amount;
    assert call.payee == payee;

    for b in script.bcs_serialize():
        print("%d " % b, end='')
    print()

def demo_p2p_script_function() -> None:
    token = aptos.TypeTag__Struct(
        value=aptos.StructTag(
            address=aptos.AccountAddress.from_bytes(b"\x00" * 15 + b"\x01"),
            module=aptos.Identifier("XDX"),
            name=aptos.Identifier("XDX"),
            type_params=[],
        )
    )
    payee = aptos.AccountAddress.from_bytes(b"\x22" * 16)
    amount = st.uint64(1_234_567)
    payload = stdlib.encode_peer_to_peer_with_metadata_script_function(token, payee, amount, b"", b"")

    call = stdlib.decode_script_function_payload(payload)
    assert isinstance(call, stdlib.ScriptFunctionCall__PeerToPeerWithMetadata)
    assert call.amount == amount;
    assert call.payee == payee;

    for b in payload.bcs_serialize():
        print("%d " % b, end='')
    print()

if __name__ == "__main__":
    demo_p2p_script()
    demo_p2p_script_function()
