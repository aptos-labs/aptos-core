# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class LoginButtonComponent < ViewComponent::Base
  include ActionText::Engine.helpers

  PROVIDER_CLASSES = {
    github: '!bg-[#24292f] !text-white',
    discord: '!bg-[#5964f2] !text-white'
  }.freeze

  def initialize(provider:,
                 **rest)
    @rest = rest
    @provider = provider
    @size = @rest.fetch(:size, :medium)
    @rest[:class] = [
      PROVIDER_CLASSES[@provider],
      @rest[:class]
    ]
  end

  private

  # Enables use of form_with helper.
  def main_app
    Rails.application.class.routes.url_helpers
  end
end
