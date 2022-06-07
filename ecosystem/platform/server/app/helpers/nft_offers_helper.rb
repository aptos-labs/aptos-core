# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

module NftOffersHelper
  def offer_dependent_logic(nft_offer)
    case nft_offer.name
    when 'nft_nyc'
      content_for(:page_title, 'NFT.NYC 2022')
    end
  end
end
