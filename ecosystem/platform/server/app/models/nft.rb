# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

# An NFT that will be minted on behalf of the user once mainnet launches.
class Nft < ApplicationRecord
  belongs_to :user
  belongs_to :nft_offer
  validates :explorer_url, format: { with: %r{\Ahttps://explorer\.devnet\.aptos\.dev/txn/\d+\?network=nft-nyc\z} },
                           allow_nil: true, allow_blank: true
end
