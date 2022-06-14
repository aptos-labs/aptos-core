# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class NftsController < ApplicationController
  before_action :authenticate_user!
  before_action :set_nft

  def show; end

  private

  def set_nft
    @nft = Nft.find(params[:id])
    head :forbidden unless @nft.user_id == current_user.id
  end
end
