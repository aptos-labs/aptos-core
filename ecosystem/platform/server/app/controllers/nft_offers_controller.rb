# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class NftOffersController < ApplicationController
  before_action :authenticate_user!
  before_action :set_nft_offer
  before_action :offer_dependent_logic

  def show; end

  def update
    nft = Nft.find_by(user: current_user, nft_offer: @nft_offer)

    # TODO: Compute image_url depending on nft_offer.name.
    image_url = "https://example.com/#{Random.uuid}"
    nft ||= Nft.create(user: current_user, nft_offer: @nft_offer, image_url:)

    # TODO: Mint the NFT on devnet and save the address.

    redirect_to nft_path(nft)
  end

  private

  def set_nft_offer
    @nft_offer = NftOffer.find(params[:id])
    return redirect_to root_path if @nft_offer.nil?

    now = DateTime.now
    valid_from = @nft_offer.valid_from
    return redirect_to root_path if valid_from && valid_from > now

    valid_until = @nft_offer.valid_until
    return redirect_to root_path if valid_until && now >= valid_until
  end

  def offer_dependent_logic
    case @nft_offer.name
    when 'nft_nyc'
      ensure_google!
    end
  end
end
