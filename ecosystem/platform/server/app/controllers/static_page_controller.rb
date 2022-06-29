# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class StaticPageController < ApplicationController
  layout 'it2', only: [:community]

  def root; end

  def community
    @login_dialog = DialogComponent.new(id: 'login_dialog')
  end

  def terms; end

  def terms_testnet; end

  def privacy; end

  def developers; end

  def currents
    @feed = Rails.cache.fetch(:currents_posts, expires_in: 1.hour) do
      rss = HTTParty.get('https://medium.com/feed/@aptoslabs')
      RSS::Parser.parse(rss.body)
    end
    @article_html = @feed.items.first.content_encoded.html_safe
  end
end
