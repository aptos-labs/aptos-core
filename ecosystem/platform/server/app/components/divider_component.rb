# frozen_string_literal: true

class DividerComponent < ViewComponent::Base
  SCHEME_CLASSES = {
    primary: 'w-full flex text-center',
    secondary: 'w-full flex'
  }.freeze

  def initialize(scheme: :secondary,
                 **rest)
    @scheme = scheme
    @rest = rest
    @rest[:class] = [
      SCHEME_CLASSES[@scheme],
      @rest[:class]
    ]
  end
end
