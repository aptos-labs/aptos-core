# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

require 'test_helper'

class WalletCreatorTest < ActiveSupport::TestCase
  test 'the wallet is created if the signature is valid' do
    signing_key = RbNaCl::SigningKey.generate
    challenge = '123456789'
    signed_challenge = signing_key.sign(challenge)

    verify_key = signing_key.verify_key
    wallet = FactoryBot.build(:wallet, public_key: "0x#{RbNaCl::Util.bin2hex(verify_key)}")

    assert_difference('Wallet.count') do
      result = WalletCreator.new.create_wallet(wallet:, challenge:, signed_challenge:)
      assert result.created?
    end
  end

  test 'the wallet is not created if the signature is invalid' do
    signing_key = RbNaCl::SigningKey.generate
    challenge = '123456789'
    signed_challenge = signing_key.sign(challenge)

    wallet = FactoryBot.build(:wallet, public_key: "0x#{Faker::Crypto.sha256}")

    assert_no_difference('Wallet.count') do
      result = WalletCreator.new.create_wallet(wallet:, challenge:, signed_challenge:)
      refute result.created?
    end
  end
end
