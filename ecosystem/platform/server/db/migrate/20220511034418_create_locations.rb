# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

# rubocop:disable Layout/LineLength
class CreateLocations < ActiveRecord::Migration[7.0]
  def change
    create_table :locations do |t|
      t.references :item, polymorphic: true, null: false

      # Location fields: https://www.rubydoc.info/gems/maxmind-geoip2/MaxMind/GeoIP2/Record/Location
      t.integer :accuracy_radius # The approximate accuracy radius in kilometers around the latitude and longitude for the IP address.
      t.integer :average_income # The average income in US dollars associated with the requested IP address.
      t.float :latitude # The approximate latitude of the location associated with the IP address.
      t.float :longitude # The approximate longitude of the location associated with the IP address.
      t.integer :metro_code # The metro code of the location if the location is in the US.
      t.integer :population_density # The estimated population per square kilometer associated with the IP address.
      t.string :time_zone # The time zone associated with location, as specified by the IANA Time Zone Database, e.g., “America/New_York”.

      # Traits fields: https://www.rubydoc.info/gems/maxmind-geoip2/MaxMind/GeoIP2/Record/Traits
      t.boolean :anonymous # This is true if the IP address belongs to any sort of anonymous network.
      t.boolean :anonymous_vpn # This is true if the IP address is registered to an anonymous VPN provider. If a VPN provider does not register subnets under names associated with them, we will likely only flag their IP ranges using the hosting_provider property.
      t.integer :autonomous_system_number # The autonomous system number associated with the IP address.
      t.string :autonomous_system_organization # The organization associated with the registered autonomous system number for the IP address.
      t.string :connection_type # The connection type may take the following values: “Dialup”, “Cable/DSL”, “Corporate”, “Cellular”.
      t.string :domain # he second level domain associated with the IP address. This will be something like “example.com” or “example.co.uk”, not “foo.example.com”.
      t.boolean :hosting_provider # This is true if the IP address belongs to a hosting or VPN provider (see description of the anonymous_vpn property).
      t.string :ip_address # The IP address that the data in the model is for. If you performed a “me” lookup against the web service, this will be the externally routable IP address for the system the code is running on. If the system is behind a NAT, this may differ from the IP address locally assigned to it.
      t.string :isp # The name of the ISP associated with the IP address.
      t.boolean :legitimate_proxy # This attribute is true if MaxMind believes this IP address to be a legitimate proxy, such as an internal VPN used by a corporation.
      t.string :mobile_country_code # The mobile country code (MCC) associated with the IP address and ISP.
      t.string :mobile_network_code # The mobile network code (MNC) associated with the IP address and ISP.
      t.string :network # The network in CIDR notation associated with the record.
      t.string :organization # The name of the organization associated with the IP address.
      t.boolean :public_proxy # This is true if the IP address belongs to a public proxy.
      t.boolean :residential_proxy # This is true if the IP address is on a suspected anonymizing network and belongs to a residential ISP.
      t.float :static_ip_score # An indicator of how static or dynamic an IP address is.
      t.boolean :tor_exit_node # This is true if the IP address is a Tor exit node.
      t.integer :user_count # The estimated number of users sharing the IP/network during the past 24 hours.
      t.string :user_type # The user type associated with the IP address. This can be one of the following values:, business, cafe, cellular, college, content_delivery_network, dialup, government, hosting, library, military, residential, router, school, search_engine_spider, traveler

      # Continent fields: https://www.rubydoc.info/gems/maxmind-geoip2/MaxMind/GeoIP2/Record/Continent
      t.string :continent_code # A two character continent code like “NA” (North America) or “OC” (Oceania).
      t.string :continent_geoname_id # The GeoName ID for the continent.
      t.string :continent_name # The first available localized name in order of preference.

      # Country fields: https://www.rubydoc.info/gems/maxmind-geoip2/MaxMind/GeoIP2/Record/Country
      t.integer :country_confidence # A value from 0-100 indicating MaxMind's confidence that the country is correct.
      t.string :country_geoname_id # The GeoName ID for the country.
      t.string :country_iso_code # The two-character ISO 3166-1 alpha code for the country.
      t.string :country_name # The first available localized name in order of preference.

      # Subdivision fields: https://www.rubydoc.info/gems/maxmind-geoip2/MaxMind/GeoIP2/Record/Subdivision
      t.integer :subdivision_confidence # This is a value from 0-100 indicating MaxMind's confidence that the subdivision is correct.
      t.string :subdivision_geoname_id # TThis is a GeoName ID for the subdivision.
      t.string :subdivision_iso_code # This is a string up to three characters long contain the subdivision portion of the ISO 3166-2 code.
      t.string :subdivision_name # The first available localized name in order of preference.

      # City fields: https://www.rubydoc.info/gems/maxmind-geoip2/MaxMind/GeoIP2/Record/City
      t.integer :city_confidence # A value from 0-100 indicating MaxMind's confidence that the city is correct.
      t.string :city_geoname_id # The GeoName ID for the city.
      t.string :city_name # The first available localized name in order of preference.

      # Postal fields: https://www.rubydoc.info/gems/maxmind-geoip2/MaxMind/GeoIP2/Record/Postal
      t.integer :postal_confidence # A value from 0-100 indicating MaxMind's confidence that the postal code is correct.
      t.string :postal_code # The postal code of the location.

      t.timestamps
    end
  end
end

# rubocop:enable Layout/LineLength
