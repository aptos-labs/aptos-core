// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

package main

import (
	"fmt"
	stdlib "testing/aptosstdlib"
	aptos "testing/aptostypes"
)

func demo_p2p_script() {
	token := &aptos.TypeTag__Struct{
		Value: aptos.StructTag{
			Address: aptos.AccountAddress(
				[16]uint8{0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1},
			),
			Module:     aptos.Identifier("XDX"),
			Name:       aptos.Identifier("XDX"),
			TypeParams: []aptos.TypeTag{},
		},
	}
	payee := aptos.AccountAddress(
		[16]uint8{0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22},
	)
	amount := uint64(1_234_567)
	script := stdlib.EncodePeerToPeerWithMetadataScript(token, payee, amount, []uint8{}, []uint8{})

	call, err := stdlib.DecodeScript(&script)
	if err != nil {
		panic(fmt.Sprintf("failed to decode script: %v", err))
	}
	payment := call.(*stdlib.ScriptCall__PeerToPeerWithMetadata)
	if payment.Amount != amount || payment.Payee != payee {
		panic("wrong script content")
	}

	bytes, err := script.BcsSerialize()
	if err != nil {
		panic("failed to serialize")
	}
	for _, b := range bytes {
		fmt.Printf("%d ", b)
	}
	fmt.Printf("\n")
}

func demo_p2p_script_function() {
	token := &aptos.TypeTag__Struct{
		Value: aptos.StructTag{
			Address: aptos.AccountAddress(
				[16]uint8{0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1},
			),
			Module:     aptos.Identifier("XDX"),
			Name:       aptos.Identifier("XDX"),
			TypeParams: []aptos.TypeTag{},
		},
	}
	payee := aptos.AccountAddress(
		[16]uint8{0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22},
	)
	amount := uint64(1_234_567)
	payload := stdlib.EncodePeerToPeerWithMetadataScriptFunction(token, payee, amount, []uint8{}, []uint8{})

	call, err := stdlib.DecodeScriptFunctionPayload(payload)
	if err != nil {
		panic(fmt.Sprintf("failed to decode script function: %v", err))
	}
	payment := call.(*stdlib.ScriptFunctionCall__PeerToPeerWithMetadata)
	if payment.Amount != amount || payment.Payee != payee {
		panic("wrong script content")
	}

	bytes, err := payload.BcsSerialize()
	if err != nil {
		panic("failed to serialize")
	}
	for _, b := range bytes {
		fmt.Printf("%d ", b)
	}
	fmt.Printf("\n")
}

func main() {
    demo_p2p_script()
    demo_p2p_script_function()
}
