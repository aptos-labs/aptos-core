# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

require 'test_helper'

class WalletVerifierTest < ActiveSupport::TestCase
  test 'the wallet is verified if the signature is valid' do
    signing_key = RbNaCl::SigningKey.generate
    verify_key = signing_key.verify_key
    wallet = FactoryBot.create(:wallet, public_key: "0x#{RbNaCl::Util.bin2hex(verify_key)}")
    challenge = '123456789'
    signed_challenge = signing_key.sign(challenge)
    WalletVerifier.new.verify_wallet(wallet, challenge, signed_challenge)
    assert wallet.verified?
  end

  test 'the wallet is not verified if the signature is nvalid' do
    signing_key = RbNaCl::SigningKey.generate
    wallet = FactoryBot.create(:wallet, public_key: "0x#{Faker::Crypto.sha256}")
    challenge = '123456789'
    signed_challenge = signing_key.sign(challenge)
    WalletVerifier.new.verify_wallet(wallet, challenge, signed_challenge)
    refute wallet.verified?
  end
end
