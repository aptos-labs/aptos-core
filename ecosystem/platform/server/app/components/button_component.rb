# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

SCHEME_CLASSES = {
  primary: 'bg-teal-400 hover:brightness-105 text-neutral-800 font-mono uppercase font-bold flex items-center justify-center',
  secondary: 'border border-teal-400 hover:border-teal-300 text-white font-mono uppercase flex items-center justify-center'
}.freeze

SIZE_CLASSES = {
  large: 'px-8 py-4 text-lg rounded gap-4',
  medium: 'px-8 py-2 text-lg rounded gap-2',
  small: 'py-1 text-sm rounded gap-1'
}.freeze

class ButtonComponent < ViewComponent::Base
  def initialize(scheme: :primary,
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
