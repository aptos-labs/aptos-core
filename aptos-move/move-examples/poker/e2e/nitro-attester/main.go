// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

package main

import (
	"encoding/base64"
	"encoding/hex"
	"errors"
	"fmt"
	"os"
	"time"

	"github.com/hf/nsm"
	"github.com/hf/nsm/request"
)

func main() {
	userDataHex := os.Getenv("USER_DATA_HEX")
	userData, err := hex.DecodeString(userDataHex)
	if err != nil {
		panic(fmt.Errorf("invalid USER_DATA_HEX: %w", err))
	}

	doc, err := attest(userData)
	if err != nil {
		panic(err)
	}

	fmt.Printf("ATTESTATION_DOC_BASE64=%s\n", base64.StdEncoding.EncodeToString(doc))
	time.Sleep(10 * time.Minute)
}

func attest(userData []byte) ([]byte, error) {
	sess, err := nsm.OpenDefaultSession()
	if err != nil {
		return nil, err
	}
	defer sess.Close()

	res, err := sess.Send(&request.Attestation{UserData: userData})
	if err != nil {
		return nil, err
	}
	if res.Error != "" {
		return nil, errors.New(string(res.Error))
	}
	if res.Attestation == nil || res.Attestation.Document == nil {
		return nil, errors.New("NSM device did not return an attestation")
	}
	return res.Attestation.Document, nil
}
