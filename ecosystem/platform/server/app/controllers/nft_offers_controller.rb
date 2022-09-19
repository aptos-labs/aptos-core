# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class NftOffersController < ApplicationController
  before_action :authenticate_user!, only: %i[create]

  def short
    @nft_offer = NftOffer.find(params[:offer_id])
    redirect_to nft_offer_path(slug: @nft_offer.slug, v: params[:txn_version])
  end

  def show
    @image_dialog = DialogComponent.new(id: 'image_dialog', class: '!w-max max-h-max')
    store_location_for(:user, request.path)
    @nft_offer = NftOffer.find_by(slug: params[:slug])
    @wallet = current_user&.wallets&.find_by(network: @nft_offer.network, public_key: params[:wallet]) ||
              Wallet.new(network: @nft_offer.network, challenge: 24.times.map { rand(10) }.join)

    @transaction_hash = params[:txn]
    @transaction_version = params[:v].to_i

    txn_hash_valid = @transaction_hash.is_a?(String) && @transaction_hash.match?(/^0x[0-9a-f]{64}$/)
    txn_version_valid = @transaction_version.positive?
    return render :minted if txn_hash_valid || txn_version_valid

    @transaction_version = nil
    @transaction_hash = nil

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

  def update
    @nft_offer = NftOffer.find_by(slug: params[:slug])
    @wallet = current_user.wallets.find_by(
      network: @nft_offer.network,
      public_key: params[:wallet],
      wallet_name: params[:wallet_name]
    )

    return render json: { error: 'wallet_not_found' } if @wallet.nil?

    result = NftClaimer.new.claim_nft(
      nft_offer: @nft_offer,
      wallet: @wallet
    )

    render json: {
      wallet_name: @wallet.wallet_name,
      module_address: @nft_offer.module_address,
      message: result.message,
      signature: result.signature
    }
  rescue NftClaimer::AccountNotFoundError
    render json: { error: 'account_not_found' }
  end

  private

  def sign_in_step
    @login_dialog = DialogComponent.new(id: 'login_dialog')
    completed = user_signed_in?
    {
      name: :sign_in,
      completed:
    }
  end

  def connect_wallet_step
    completed = user_signed_in? && @wallet.persisted? && @wallet.network == @nft_offer.network
    {
      name: :connect_wallet,
      completed:
    }
  end

  def claim_nft_step
    completed = false
    {
      name: :claim_nft,
      completed:
    }
  end
end
