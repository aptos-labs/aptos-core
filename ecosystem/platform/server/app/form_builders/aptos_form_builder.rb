# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class AptosFormBuilder < ActionView::Helpers::FormBuilder
  TEXT_FIELDS = %i[text_field text_area email_field url_field date_field].freeze
  TEXT_FIELDS.each do |field|
    define_method field do |method, options = {}|
      options[:class] = [
        'text-neutral-100 placeholder:text-neutral-400 bg-neutral-900 ' \
        'border-neutral-600 ring-0 hover:border-teal-800 appearance-none rounded-lg ' \
        'w-full p-4 focus:ring-teal-700 focus:border-teal-700',
        { 'ring-red-500': @object && @object.errors[method].present? },
        options[:class]
      ]
      super(method, options)
    end
  end

  def check_box(method, options = {})
    options[:class] = [
      'bg-transparent border-teal-400 checked:bg-teal-400 hover:checked:bg-teal-400 focus:checked:bg-teal-400 ' \
      'rounded focus:ring-0 outline-none',
      options[:class]
    ]
    super(method, options)
  end

  def submit(value = nil, options = {})
    options[:type] = :submit
    @template.render(ButtonComponent.new(**options)) { value }
  end
end
