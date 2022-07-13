# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class HealthController < ApplicationController
  def health
    User.where(id: 1).exists?
    render plain: '🔥🤼🔥'
  end
end
