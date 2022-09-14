# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

require 'rmagick'
require 'open3'

MAX_IMAGE_NUM_ALLOWED = 100_000_000

class NftImagesController < ApplicationController
  before_action :validate_params
  include ActiveStorage::SetCurrent

  def show
    @nft_offer = NftOffer.find_by(slug: params[:nft_offer_slug])
    @image_num = params[:image_num].to_i
    unless @image_num.positive? && @image_num <= MAX_IMAGE_NUM_ALLOWED
      render plain: nil,
             status: :bad_request
      return
    end

    image = NftImage.find_or_create(@nft_offer.slug, @image_num)

    redirect_to rails_blob_path(image.image, disposition: 'attachment') # image.image.url(expires_in: 7.days)
    # send_file compress_image(template_img), status: 200, content_type: 'image/png',
    #                                        filename:
  end

  protected

  def validate_params
    return false unless params[:filter].present? && params[:post_type].present?

    params_include?(:filter, %w[popular following picks promoted first-posts])
    params_include?(:post_type, %w[discussions snaps code links])
  end

  # TODO: WRITE THIS TO GCS FIRST, USING ACTIVESTORAGE
  # def check_cache
  #  cache_path = "#{@nft_offer.network}/nft_offer/#{@nft_offer.slug}/images/#{@image_num}"
  # end
end
