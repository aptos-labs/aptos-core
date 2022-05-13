# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class AptosFormBuilder < ActionView::Helpers::FormBuilder
  TEXT_FIELDS = %i[text_field email_field].freeze
  TEXT_FIELDS.each do |field|
    define_method field do |method, options = {}|
      options[:class] = [
        'font-mono placeholder:text-white text-lg bg-transparent appearance-none border border-white rounded '\
        'w-full py-2 px-4 focus:ring-0 focus:border-teal-400',
        { 'border-red-500': @object.errors[method].present? },
        options[:class]
      ]
      super(method, options)
    end
  end

  def check_box(method, options = {})
    options[:class] = [
      'bg-transparent border-teal-400 checked:bg-teal-400 hover:checked:bg-teal-400 focus:checked:bg-teal-400 '\
      'focus:ring-0 outline-none',
      options[:class]
    ]
    super(method, options)
  end

  def submit(value = nil, options = {})
    options[:type] = :submit
    @template.render(ButtonComponent.new(**options)) { value }
  end
end
