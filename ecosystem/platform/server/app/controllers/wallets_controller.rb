# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class WalletsController < ApplicationController
  before_action :authenticate_user!

  def show
    @wallet = Wallet.find(params[:id])
    render ConnectWalletButtonComponent.new(wallet: @wallet)
  end

  def create
    wallet_params = params.require(:wallet).permit(
      :network, :wallet_name, :public_key,
      :challenge, :signed_challenge
    )

    wallet = Wallet.new(wallet_params)
    wallet.user = current_user

    result = WalletCreator.new.create_wallet(
      wallet:
    )

    if result.created?
      stored_location = stored_location_for(current_user)
      if stored_location
        stored_location = if '?' in stored_location
                            "#{stored_location}&"
                          else
                            "#{stored_location}?"
                          end
        stored_location += "wallet=#{wallet.public_key}"
        redirect_to stored_location
      else
        redirect_to result.wallet
      end
    else
      render turbo_stream: turbo_stream.replace(:connect_wallet, ConnectWalletButtonComponent
        .new(wallet: result.wallet)
        .render_in(view_context))
    end
  end
end
