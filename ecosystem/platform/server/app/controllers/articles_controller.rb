# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class ArticlesController < ApplicationController
  def index
    @articles = Article.where(status: 'published').order(created_at: :desc)
  end

  def show
    @article = Article.find_by(slug: params[:slug], status: 'published')
  end
end
