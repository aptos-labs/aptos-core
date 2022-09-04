# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class WalletCreator
  def create_wallet(wallet:)
    return Result.new(created: false, wallet:) unless wallet.valid?

    verify_key = Ed25519::VerifyKey.new(wallet.public_key_bytes)

    begin
      verify_key.verify(wallet.signed_challenge_bytes, wallet.challenge)
      wallet.save
    rescue Ed25519::VerifyError
      wallet.errors.add :signed_challenge, 'could not be verified'
    end

    Result.new(created: wallet.persisted?, wallet:)
  end

  class Result
    attr_reader :wallet

    def initialize(created:, wallet:)
      @created = created
      @wallet = wallet
    end

    def created?
      @created
    end
  end
end
