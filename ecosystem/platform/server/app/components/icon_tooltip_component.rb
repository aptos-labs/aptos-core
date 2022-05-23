# frozen_string_literal: true

class IconTooltipComponent < ViewComponent::Base
  renders_one :header
  renders_one :body

  def initialize(icon, size: :small, **rest)
    @icon = icon
    @size = size
    @rest = rest
  end
end
