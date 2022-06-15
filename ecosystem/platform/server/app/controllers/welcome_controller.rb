# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class WelcomeController < ApplicationController
  layout 'it2'

  def index
    @login_dialog = DialogComponent.new
  end
end
