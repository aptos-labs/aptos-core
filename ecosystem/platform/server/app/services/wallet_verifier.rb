# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class WalletVerifier
  def verify_wallet(wallet, challenge, signed_challenge)
    public_key_bytes = RbNaCl::Util.hex2bin(wallet.public_key[2..])
    verify_key = RbNaCl::VerifyKey.new(public_key_bytes)

    verified = begin
      verify_key.verify(signed_challenge, challenge)
      true
    rescue RbNaCl::BadSignatureError
      false
    end

    wallet.update(verified:)
    wallet
  end
end
