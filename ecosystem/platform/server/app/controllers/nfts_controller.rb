# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class NftsController < ApplicationController
  before_action :authenticate_user!
  before_action :set_nft

  def show; end

  def update
    nft_params = params.fetch(:nft, {}).permit(:explorer_url)
    if @nft.update(nft_params)
      redirect_to nft_path(@nft)
    else
      render :show, status: :unprocessable_entity
    end
  end

  private

  def set_nft
    @nft = Nft.find(params[:id])
    head :forbidden unless @nft.user_id == current_user.id
  end
end
