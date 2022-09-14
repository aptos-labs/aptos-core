# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class WalletsController < ApplicationController
  before_action :authenticate_user!

  def create
    wallet_params = params.require(:wallet).permit(
      :network, :wallet_name, :public_key,
      :challenge, :signed_challenge
    )

    wallet = Wallet.find_by(
      user: current_user,
      public_key: wallet_params[:public_key],
      network: wallet_params[:network]
    )
    return render json: { created: true, errors: [] } if wallet

    wallet = Wallet.new(wallet_params)
    wallet.user = current_user

    result = WalletCreator.new.create_wallet(
      wallet:
    )

    render json: { created: result.created?, errors: result.wallet.errors.map(&:full_message) }
  end
end
