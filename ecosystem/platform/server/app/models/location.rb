# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0
class Location < ApplicationRecord
  belongs_to :item, polymorphic: true

  # @param [MaxMind::GeoIP2::Model::Insights] data
  def self.upsert_from_maxmind!(item, data)
    item.location = (item&.location || Location.new).tap { |location| location.assign_from_maxmind(data) }
    item.location.save!
  end

  # @param [MaxMind::GeoIP2::Model::Insights] data
  def assign_from_maxmind(data)
    attr = {}

    if (location = data.location)
      attr.merge!({
                    accuracy_radius: location.accuracy_radius,
                    average_income: location.average_income,
                    latitude: location.latitude,
                    longitude: location.longitude,
                    metro_code: location.metro_code,
                    population_density: location.population_density,
                    time_zone: location.time_zone
                  })
    end

    if (traits = data.traits)
      attr.merge!({
                    anonymous: traits.anonymous?,
                    anonymous_vpn: traits.anonymous_vpn?,
                    autonomous_system_number: traits.autonomous_system_number,
                    autonomous_system_organization: traits.autonomous_system_organization,
                    connection_type: traits.connection_type,
                    domain: traits.domain,
                    hosting_provider: traits.hosting_provider?,
                    ip_address: traits.ip_address,
                    isp: traits.isp,
                    legitimate_proxy: traits.legitimate_proxy?,
                    mobile_country_code: traits.mobile_country_code,
                    mobile_network_code: traits.mobile_network_code,
                    network: traits.network,
                    organization: traits.organization,
                    public_proxy: traits.public_proxy?,
                    residential_proxy: traits.residential_proxy?,
                    static_ip_score: traits.static_ip_score,
                    tor_exit_node: traits.tor_exit_node?,
                    user_count: traits.user_count,
                    user_type: traits.user_type
                  })
    end

    if (continent = data.continent)
      attr.merge!({
                    continent_code: continent.code,
                    continent_geoname_id: continent.geoname_id,
                    continent_name: continent.name
                  })
    end

    if (country = data.country)
      attr.merge!({
                    country_confidence: country.confidence,
                    country_geoname_id: country.geoname_id,
                    country_iso_code: country.iso_code,
                    country_name: country.name
                  })
    end

    if (subdivision = data.subdivisions&.first)
      attr.merge!({
                    subdivision_confidence: subdivision.confidence,
                    subdivision_geoname_id: subdivision.geoname_id,
                    subdivision_iso_code: subdivision.iso_code,
                    subdivision_name: subdivision.name
                  })
    end

    if (subdivision = data.subdivisions&.first)
      attr.merge!({
                    subdivision_confidence: subdivision.confidence,
                    subdivision_geoname_id: subdivision.geoname_id,
                    subdivision_iso_code: subdivision.iso_code,
                    subdivision_name: subdivision.name
                  })
    end

    if (city = data.city)
      attr.merge!({
                    city_confidence: city.confidence,
                    city_geoname_id: city.geoname_id,
                    city_name: city.name
                  })
    end

    if (postal = data.postal)
      attr.merge!({
                    postal_confidence: postal.confidence,
                    postal_code: postal.code
                  })
    end

    assign_attributes attr
  end
end
