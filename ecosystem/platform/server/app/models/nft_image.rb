# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class NftImageQueryHelper
  def self._find_or_create(nft_offer, image_num)
    loop do
      if nft_offer.distinct_images
        query = NftImage.where(slug: nft_offer.slug, image_number: image_num)
      else
        image_num = 1
        query = NftImage.where(slug: nft_offer.slug)
      end

      is_new = false
      img = query.first_or_create! do |image|
        is_new = true
        image.slug = nft_offer.slug
        image.image_number = image_num
      end

      return img, is_new
    rescue PG::UniqueViolation
      # We retry on conflict!
    end
  end
end

class NftImage < ApplicationRecord
  has_one_attached :image
  default_scope { with_attached_image }

  def self.find_or_create(slug, image_num)
    # Verify this is a valid slug
    nft_offer = NftOffer.find_by(slug:)
    # Find or create the nft_image
    img, is_new = NftImageQueryHelper._find_or_create(nft_offer, image_num)

    image_file = nil
    if (is_new || !img.image&.attached?) && !img.image&.reload&.attached?
      filename = if nft_offer.distinct_images
                   "#{nft_offer.network}__#{nft_offer.slug.underscore}__#{image_num}.png"
                 else
                   "#{nft_offer.network}__#{nft_offer.slug.underscore}.png"
                 end
      image_file = create_image(slug, image_num)
      image_file.rewind
      img.image.attach io: File.open(image_file.path), filename:, content_type: 'image/png', identify: false
    end

    img
  ensure
    # Clean up image_file if it exists
    begin
      if image_file.present?
        image_file.close
        image_file.unlink
      end
    rescue StandardError
      # Ignored
    end
  end

  # Returns a `TempFile` with the image contents
  def self.create_image(slug, image_num)
    case NftOffer.find_by(slug:).slug
    when NftOffer::APTOS_ZERO
      create_aptos_zero_image image_num
    end
  end

  def self.create_aptos_zero_image(image_num)
    image_path = Rails.root.join('app', 'assets', 'images', 'aptos_nft_zero_08302022.png').to_s
    template_img = Magick::Image.read(image_path).first

    overlay = Magick::Draw.new

    # Font size, fill, font
    overlay.pointsize = 29
    overlay.fill = '#FFFFFF'
    overlay.font = Rails.root.join('app', 'assets', 'fonts', 'lft_etica_mono-light.otf').to_s

    overlay.gravity = Magick::NorthWestGravity

    image_text = format '#%09d', image_num

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
    compress_image template_img
  end

  # Compresses the image, and returns a `TempFile` with the image contents
  def self.compress_image(imagemagick_image)
    source = Tempfile.new(encoding: 'binary')
    source.write(imagemagick_image.to_blob)
    source.flush

    optimized = Tempfile.new([@image_num.to_i.to_s, '.png'], encoding: 'binary')
    optimized.flush

    _out, err, _t = Open3.capture3('pngquant', '--force', '--output', optimized.path, '--', source.path)
    Rails.logger.warn("Error writing image: #{err}") if err.present?
    optimized.rewind
    optimized
  end

  def nft_offer
    NftOffer.find_by(slug:)
  end
end
