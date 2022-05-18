# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0
# frozen_string_literal: true

class IconComponent < ViewComponent::Base
  SIZE_CLASSES = {
    large: 'w-8 h-8',
    medium: 'w-6 h-6',
    small: 'w-4 h-4'
  }.freeze

  ICONS = Dir[File.join(Rails.root, 'app/assets/images/icons/*.svg')].to_h do |icon_path|
    icon_name, _ext = File.basename(icon_path).split('.')
    [icon_name.to_sym, icon_path]
  end

  def initialize(icon, size: :unspecified, **rest)
    raise 'Invalid icon - restart the server if you added one.' unless ICONS.include? icon

    @rest = rest
    @rest[:class] = [
      SIZE_CLASSES[size],
      rest[:class]
    ]
    @icon = icon
  end

  def svg
    path = ICONS[@icon]
    Rails.cache.fetch(icon: @icon) { File.read(path).html_safe }
  end

  def call
    content_tag :div, svg, **@rest
  end
end
