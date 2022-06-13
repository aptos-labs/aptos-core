# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

module NftHelper
  def nft_image_url(nft)
    case nft.nft_offer.name
    when 'nft_nyc'
      'https://aptos-community.s3.us-west-2.amazonaws.com/nyc_nft_nft_demo.jpg'
    end
  end
end
