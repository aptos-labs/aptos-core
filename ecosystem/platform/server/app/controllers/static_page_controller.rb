# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class StaticPageController < ApplicationController
  layout 'it2', only: [:community]

  def root; end

  def community
    @login_dialog = DialogComponent.new(id: 'login_dialog')
  end
end
