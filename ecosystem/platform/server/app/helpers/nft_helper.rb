# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

module NftHelper
  def nft_image_url(nft)
    case nft.nft_offer.name
    when 'nft_nyc'
      # TODO: Implement the real image for nft_nyc.
      'https://picsum.photos/500'
    end
  end
end
