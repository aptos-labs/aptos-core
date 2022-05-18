# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0
module LeaderboardHelper
  def availability_color(availability)
    if availability >= 97
      'text-green-400'
    elsif availability >= 95
      'text-orange-400'
    else
      'text-red-400'
    end
  end
end
