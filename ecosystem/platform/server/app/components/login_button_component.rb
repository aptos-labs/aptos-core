# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

PROVIDER_CLASSES = {
  github: '!bg-[#242930]',
  discord: '!bg-[#5965f2]'
}.freeze

ICON_CLASSES = {
  large: 'w-8 h-8',
  medium: 'w-6 h-6',
  small: 'w-4 h-4'
}.freeze

class LoginButtonComponent < ViewComponent::Base
  include ActionText::Engine.helpers

  def initialize(provider:,
                 **rest)
    @provider = provider
    rest[:class] = [
      PROVIDER_CLASSES[@provider],
      '!text-white',
      rest[:class]
    ]
    @rest = rest
  end

  private

  # Enables use of form_with helper.
  def main_app
    Rails.application.class.routes.url_helpers
  end

  def icon_class
    ICON_CLASSES[@rest.fetch(:size, :medium)]
  end
end
