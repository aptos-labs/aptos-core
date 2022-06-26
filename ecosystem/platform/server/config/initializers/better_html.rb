# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

BetterHtml.config = BetterHtml::Config.new(YAML.load(File.read(Rails.root.join('.better-html.yml'))))
