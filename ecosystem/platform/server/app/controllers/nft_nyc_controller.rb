# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class NftNycController < ApplicationController
  def show
    nft_offer = NftOffer.find_or_create_by(name: 'nft_nyc')
    store_location_for(:user, nft_offer_path(nft_offer))
  end
end
