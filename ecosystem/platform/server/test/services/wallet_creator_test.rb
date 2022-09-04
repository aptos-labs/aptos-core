# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

require 'test_helper'

class WalletCreatorTest < ActiveSupport::TestCase
  test 'the wallet is created if the signature is valid' do
    signing_key = Ed25519::SigningKey.generate

    verify_key = signing_key.verify_key
    wallet = FactoryBot.build(:wallet, public_key: "0x#{verify_key.to_bytes.unpack1('H*')}")
    wallet.challenge = '1' * 24
    wallet.signed_challenge = "0x#{signing_key.sign(wallet.challenge).unpack1('H*')}"

    assert_difference('Wallet.count') do
      result = WalletCreator.new.create_wallet(wallet:)
      assert result.created?
    end
  end

  test 'the wallet is not created if the signature is invalid' do
    signing_key = Ed25519::SigningKey.generate

    wallet = FactoryBot.build(:wallet, public_key: "0x#{Faker::Crypto.sha256}")
    wallet.challenge = '1' * 24
    wallet.signed_challenge = "0x#{signing_key.sign(wallet.challenge).unpack1('H*')}"

    assert_no_difference('Wallet.count') do
      result = WalletCreator.new.create_wallet(wallet:)
      refute result.created?
    end
  end
end
