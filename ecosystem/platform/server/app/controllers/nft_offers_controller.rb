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
    nft ||= Nft.create(user: current_user, nft_offer: @nft_offer)
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
      redirect_to nft_nyc_path unless current_user.authorizations.where(provider: :google).exists?
    end
  end
end
