# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class NftOffersController < ApplicationController
  def show
    store_location_for(:user, request.path)
    slug = params.require(:slug)
    @nft_offer = get_nft_offer(slug)
    @steps = [
      sign_in_step,
      connect_wallet_step,
      claim_nft_step
    ].map do |h|
      # rubocop:disable Style/OpenStructUse
      OpenStruct.new(**h)
      # rubocop:enable Style/OpenStructUse
    end
    first_incomplete = @steps.index { |step| !step.completed }
    @steps[first_incomplete + 1..].each { |step| step.disabled = true } if first_incomplete
  end

  private

  def get_nft_offer(slug)
    case slug
    when 'aptos-zero'
      NftOffer.new(slug: 'aptos-zero', network: 'devnet')
    else
      raise ActiveRecord::RecordNotFound
    end
  end

  def sign_in_step
    completed = user_signed_in?
    {
      name: :sign_in,
      completed:,
      href: new_user_session_path
    }
  end

  def connect_wallet_step
    completed = user_signed_in? && current_user.wallets.where(network: @nft_offer.network).exists?
    {
      name: :connect_wallet,
      completed:,
      dialog: completed ? nil : DialogComponent.new
    }
  end

  def claim_nft_step
    completed = false
    {
      name: :claim_nft,
      completed:,
      dialog: completed ? nil : DialogComponent.new
    }
  end
end
