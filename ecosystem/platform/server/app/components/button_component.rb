# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

SCHEME_CLASSES = {
  primary: 'bg-surf-400 hover:bg-surf-300 text-surf-900 text-center font-mono uppercase font-bold',
  secondary: 'border border-surf-400 hover:border-surf-300 text-center text-white font-mono uppercase'
}.freeze

SIZE_CLASSES = {
  large: 'px-8 py-4 text-xl rounded-lg',
  medium: 'p-2 text-lg rounded-lg',
  small: 'py-1 text-sm rounded-lg'
}.freeze

class ButtonComponent < ViewComponent::Base
  def initialize(scheme: :primary, # rubocop:disable Lint/MissingSuper
                 size: :medium,
                 **rest)
    rest[:class] = [
      SCHEME_CLASSES[scheme],
      SIZE_CLASSES[size],
      rest[:class]
    ]
    @rest = rest
    @tag = rest[:href] ? :a : :button
  end
end
