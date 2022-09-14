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
    message = WalletCreator.new.send(:verify_wallet_message, wallet.challenge)
    wallet.signed_challenge = "0x#{signing_key.sign(message).unpack1('H*')}"

    assert_difference('Wallet.count') do
      result = WalletCreator.new.create_wallet(wallet:)
      assert result.created?
    end
  end

  test 'the wallet is not created if the signature is invalid' do
    signing_key = Ed25519::SigningKey.generate

    wallet = FactoryBot.build(:wallet, public_key: "0x#{Faker::Crypto.sha256}")
    wallet.challenge = '1' * 24
    message = WalletCreator.new.send(:verify_wallet_message, wallet.challenge)
    wallet.signed_challenge = "0x#{signing_key.sign(message).unpack1('H*')}"

    assert_no_difference('Wallet.count') do
      result = WalletCreator.new.create_wallet(wallet:)
      refute result.created?
    end
  end

  test 'wallets with the same address but different wallet_name can be created' do
    signing_key = Ed25519::SigningKey.generate

    verify_key = signing_key.verify_key
    public_key = "0x#{verify_key.to_bytes.unpack1('H*')}"
    challenge = '1' * 24
    message = WalletCreator.new.send(:verify_wallet_message, challenge)
    signed_challenge = "0x#{signing_key.sign(message).unpack1('H*')}"

    petra_wallet = FactoryBot.build(:wallet, wallet_name: 'petra', public_key:, challenge:, signed_challenge:)
    martian_wallet = FactoryBot.build(:wallet, wallet_name: 'martian', public_key:, challenge:, signed_challenge:)

    assert_difference('Wallet.count', 2) do
      result = WalletCreator.new.create_wallet(wallet: petra_wallet)
      assert result.created?

      result = WalletCreator.new.create_wallet(wallet: martian_wallet)
      assert result.created?
    end
  end
end
