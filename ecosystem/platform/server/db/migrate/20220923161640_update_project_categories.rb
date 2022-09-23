# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class UpdateProjectCategories < ActiveRecord::Migration[7.0]
  def change
    Category.find_by(title: 'Tooling').update(title: 'Dev Tooling')
    Category.find_by(title: 'Data').update(title: 'Data & Analytics')
    Category.find_by(title: 'Lending').destroy
    Category.find_by(title: 'Other').destroy

    Category.create(title: 'Infrastructure')
    Category.create(title: 'Identity')
    Category.create(title: 'Storage')
    Category.create(title: 'Block Explorers')
    Category.create(title: 'Security')
    Category.create(title: 'Monitoring')
    Category.create(title: 'Oracles')
    Category.create(title: 'Protocols')
    Category.create(title: 'Bridges')
    Category.create(title: 'APIs')
    Category.create(title: 'Payments')
    Category.create(title: 'Social Media')
    Category.create(title: 'Education')
    Category.create(title: 'DAOs')
  end
end
