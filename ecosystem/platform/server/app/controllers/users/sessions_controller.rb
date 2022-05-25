# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

module Users
  class SessionsController < Devise::SessionsController
    layout 'it1', only: %i[new]
  end
end
