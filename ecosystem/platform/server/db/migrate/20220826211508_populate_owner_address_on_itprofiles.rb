# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class PopulateOwnerAddressOnItprofiles < ActiveRecord::Migration[7.0]
  # The following was already run asynchronously in staging/prod, to prevent downtime
  def up
    $stdout.sync = true
    ActiveRecord::Base.logger = Logger.new($stdout, level: Logger::INFO)

    profile_query = It3Profile.where.not(owner_key: nil).where(owner_address: nil)
    profile_count = profile_query.count
    puts "#{profile_count} #{It3Profile.to_s.pluralize} to process"
    total_processed = 0
    profile_query.find_in_batches.map do |group|
      group.each { |ait| ait.update_columns(owner_address: ApplicationRecord.address_from_key(ait.owner_key)) }
      total_processed += group.length
      puts "Processed #{total_processed}/#{profile_count} #{It3Profile.to_s.pluralize}"
    end
  end

  def down; end
end
