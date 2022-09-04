# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class NftClaimer
  def claim_nft(nft_offer:, wallet:)
    message = build_claim_message(nft_offer, wallet)
    signing_key = Ed25519::SigningKey.new(nft_offer.private_key_bytes)

    signature_bytes = signing_key.sign(message)
    signature = "0x#{signature_bytes.unpack1('H*')}"

    Result.new(message:, signature:)
  end

  class AccountNotFoundError < StandardError; end

  class Result
    attr_reader :message, :signature

    def initialize(message:, signature:)
      @message = message
      @signature = signature
    end
  end

  private

  def build_claim_message(nft_offer, wallet)
    [
      "#{nft_offer.module_address}::Anchor",
      wallet.address,
      get_sequence_number(wallet)
    ].join('!')
  end

  def get_sequence_number(wallet)
    account_url = [wallet.api_url, 'accounts', wallet.address].join('/')
    response = HTTParty.get(account_url)

    raise AccountNotFoundError unless response.success?

    response['sequence_number'].to_i
  end
end
