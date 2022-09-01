# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class WalletCreator
  def create_wallet(wallet, challenge, signed_challenge)
    public_key_bytes = RbNaCl::Util.hex2bin(wallet.public_key[2..])
    verify_key = RbNaCl::VerifyKey.new(public_key_bytes)

    begin
      verify_key.verify(signed_challenge, challenge)
      wallet.save
    rescue RbNaCl::BadSignatureError
      # The signature was invalid; therefore, don't save the wallet.
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
