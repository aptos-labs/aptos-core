# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class PopulateAccountAddressOnItprofiles < ActiveRecord::Migration[7.0]
  # The following was already run asynchronously in staging/prod, to prevent downtime
  def up
    $stdout.sync = true
    ActiveRecord::Base.logger = Logger.new($stdout, level: Logger::INFO)

    [It1Profile, It2Profile].each do |ait_klass|
      profile_query = ait_klass.where.not(account_key: nil).where(account_address: nil)
      profile_count = profile_query.count
      puts "#{profile_count} #{ait_klass.to_s.pluralize} to process"
      total_processed = 0
      profile_query.find_in_batches.map do |group|
        group.each { |ait| ait.update_columns(account_address: ApplicationRecord.address_from_key(ait.account_key)) }
        total_processed += group.length
        puts "Processed #{total_processed}/#{profile_count} #{ait_klass.to_s.pluralize}"
      end
    end
  end

  def down; end
end
