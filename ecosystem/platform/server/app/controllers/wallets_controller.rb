# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class WalletsController < ApplicationController
  before_action :authenticate_user!

  def create
    wallet_params = params.require(:wallet).permit(
      :network, :wallet_name, :public_key
    )

    challenge = params.require(:challenge)
    return head :forbidden unless challenge.match(/[0-9]{24}/)

    signed_challenge_hex = params.require(:signed_challenge)
    return head :forbidden unless signed_challenge_hex.match(/[0-9a-f]{128}/)

    signed_challenge = RbNaCl::Util.hex2bin(signed_challenge_hex)

    wallet = Wallet.new(wallet_params)
    wallet.user = current_user

    result = WalletCreator.new.create_wallet(
      wallet:,
      challenge:,
      signed_challenge:
    )

    render json: { created: result.created? }
  end
end
