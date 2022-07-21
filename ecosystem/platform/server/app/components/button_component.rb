# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class ButtonComponent < ViewComponent::Base
  SCHEME_CLASSES = {
    primary: 'bg-teal-400 hover:brightness-105 text-neutral-800 font-mono uppercase font-bold flex ' \
             'items-center justify-center disabled:opacity-50',
    secondary: 'border border-teal-400 hover:border-teal-300 text-white font-mono uppercase flex ' \
               'items-center justify-center'
  }.freeze

  SIZE_CLASSES = {
    large: 'px-8 py-4 text-lg rounded-lg gap-4',
    medium: 'px-8 py-2 text-lg rounded-lg gap-2',
    small: 'px-4 py-1 text-sm rounded-lg gap-1'
  }.freeze

  def initialize(scheme: :primary,
                 size: :medium,
                 dialog: nil,
                 **rest)
    @rest = rest
    @rest[:class] = [
      SCHEME_CLASSES[scheme],
      SIZE_CLASSES[size],
      @rest[:class]
    ]
    @rest[:onclick] = "document.getElementById('#{dialog.id}').showModal()" if dialog
    @tag = @rest[:href] ? :a : :button
  end

  def call
    content_tag @tag, content, **@rest
  end
end
