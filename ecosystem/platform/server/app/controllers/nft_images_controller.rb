# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

require 'rmagick'
require 'open3'

MAX_IMAGE_NUM_ALLOWED = 100_000_000

class NftImagesController < ApplicationController
  before_action :validate_params

  def show
    @nft_offer = NftOffer.find(params[:nft_offer_slug])
    @image_num = params[:image_num].to_i
    unless @image_num.positive? && @image_num <= MAX_IMAGE_NUM_ALLOWED
      render plain: nil,
             status: :bad_request
      return
    end

    case @nft_offer.slug
    when NftOffer::APTOS_ZERO
      image_path = Rails.root.join('app', 'assets', 'images', 'aptos_nft_zero_08302022.png').to_s
      font_path = Rails.root.join('app', 'assets', 'fonts', 'lft_etica_mono-light.otf').to_s
      template_img = Magick::Image.read(image_path).first

      overlay = Magick::Draw.new
      # The font size
      # This one needs a bit more information. Will explain it below
      overlay.gravity = Magick::NorthWestGravity

      # Font size, fill, font
      overlay.pointsize = 29
      overlay.fill = '#FFFFFF'
      overlay.font = font_path

      image_text = format '#%09d', @image_num

      metrics = overlay.get_multiline_type_metrics(template_img, image_text)
      #   #<struct Magick::TypeMetric
      #    pixels_per_em=#<struct Magick::Point x=29, y=29>,
      #    ascent=29.0,
      #    descent=-8.0,
      #    width=396.0,
      #    height=37.0,
      #    max_advance=18.0,
      #    bounds=#<struct Magick::Segment x1=0.0, y1=-6.0, x2=16.328125, y2=21.0>,
      #    underline_position=-6.699000000000001,
      #    underline_thickness=-6.699000000000001>

      overlay.annotate(template_img, 0, 0, 512.5 - (metrics.width / 2), 980 - metrics.height, image_text)
    else
      raise ActiveRecord::RecordNotFound
    end

    filename = "#{@nft_offer.network}__#{@nft_offer.slug.underscore}__#{@image_num}.png"
    send_file compress_image(template_img), status: 200, content_type: 'image/png',
                                            filename:
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

  def compress_image(image)
    source = Tempfile.new(encoding: 'binary')
    source.write(image.to_blob)
    source.flush

    optimized = Tempfile.new([@image_num.to_i.to_s, '.png'], encoding: 'binary')
    optimized.flush

    _out, err, _t = Open3.capture3('pngquant', '--force', '--output', optimized.path, '--', source.path)
    Rails.logger.warn("Error writing image: #{err}") if err.present?
    optimized.rewind
    optimized
  end
end
