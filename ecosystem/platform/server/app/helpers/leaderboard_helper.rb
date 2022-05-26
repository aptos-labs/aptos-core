# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0
module LeaderboardHelper
  def availability_color(availability)
    if availability >= 97
      'bg-teal-400'
    elsif availability >= 95
      'bg-yellow-500'
    else
      'bg-red-500'
    end
  end

  def liveness_icon(liveness)
    if liveness >= 97
      render IconComponent.new(:check_circle, class: 'text-teal-400 w-5 h-5')
    elsif liveness >= 95
      render IconComponent.new(:check_circle, class: 'text-yellow-500 w-5 h-5')
    else
      render IconComponent.new(:x_circle, class: 'text-red-500 w-5 h-5')
    end
  end
end
