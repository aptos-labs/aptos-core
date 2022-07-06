# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

# This controller is for pages that are completely static (i.e. they render the
# same HTML every time regardless of session state). These pages can therefore
# be cached and served from the CDN.
#
# If a page does need limited dynamic content (e.g. log in / log out button),
# the page should render the common case (e.g. log in button) and load the
# correct content via turbo-frame.
class StaticPageController < ApplicationController
  layout 'it2', only: [:community]
  before_action :set_cache_headers

  def community
    @login_dialog = DialogComponent.new(id: 'login_dialog')
  end

  private

  def set_cache_headers
    # Disabling this temporarily to unblock flash on main page
    # expires_in 1.hour, public: true
  end
end
