# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class ButtonComponent < ViewComponent::Base
  SCHEME_CLASSES = {
    primary: 'bg-teal-400 hover:brightness-105 text-neutral-800 font-mono uppercase flex ' \
             'items-center justify-center disabled:opacity-50 subpixel-antialiased font-normal ' \
             'active:brightness-95 whitespace-nowrap',
    secondary: 'whitespace-nowrap bg-transparent ring-1 text-teal-300 ring-teal-400 hover:ring-2 ' \
               'hover:bg-neutral-800 hover:ring-teal-500 hover:text-teal-400 ' \
               'uppercase font-normal uppercase font-mono active:brightness-95 font-normal ' \
               'hover:subpixel-antialiased ring-inset',
    tertiary: 'whitespace-nowrap text-teal-300 hover:brightness-105 active:brightness-95 uppercase ' \
              'font-normal font-mono',
    blank: '!p-0 hover:brightness-110'
  }.freeze

  SIZE_CLASSES = {
    xl: 'px-24 py-6 text-2xl rounded-lg gap-8',
    large: 'px-12 py-4 text-xl rounded-lg gap-4',
    medium: 'px-8 py-2 text-lg rounded-lg gap-2',
    small: 'px-4 py-1.5 text-sm rounded-lg gap-1',
    tiny: 'px-3 py-1 text-xs rounded gap-1'
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
