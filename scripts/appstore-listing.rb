#!/usr/bin/env ruby
# Push the listing copy in appstore/metadata to App Store Connect.
#
# The copy lives in text files rather than in here so that changing a description is a diff someone
# can read, and so the store listing is reviewable in the same place as the code it describes.
#
#   APPLE_EMAIL=you@example.com ruby scripts/appstore-listing.rb
#
# Run it yourself: the Apple ID login prompts for a two-factor code the first time each month.
begin
  require "spaceship"
rescue LoadError
  libexec = Dir["/opt/homebrew/Cellar/fastlane/*/libexec", "/usr/local/Cellar/fastlane/*/libexec"].max
  abort "spaceship is not installed. `brew install fastlane` and rerun." unless libexec
  ENV["GEM_PATH"] = [libexec, ENV["GEM_PATH"]].compact.join(":")
  Gem.clear_paths
  require "spaceship"
end

require "json"

DIR = ENV.fetch("METADATA_DIR", "appstore/metadata")
LOCALE = "en-US"
BUNDLE_ID = ENV.fetch("BUNDLE_ID", "studio.margin.app")

# This Apple ID can see more than one App Store Connect team, and the wrong one belongs to someone
# else entirely. Pin it rather than letting spaceship pick.
ITC_TEAM_ID = ENV.fetch("FASTLANE_ITC_TEAM_ID", "129377371")
ENV["FASTLANE_ITC_TEAM_ID"] = ITC_TEAM_ID

# Apple rejects an over-length field with a validation error that does not name the limit, so the
# check belongs here where the number is visible.
LIMITS = {
  "name" => 30, "subtitle" => 30, "keywords" => 100,
  "promotional_text" => 170, "description" => 4000,
}.freeze

def field(name, localized: true)
  path = localized ? File.join(DIR, LOCALE, "#{name}.txt") : File.join(DIR, "#{name}.txt")
  return nil unless File.exist?(path)

  value = File.read(path).strip
  limit = LIMITS[name]
  abort "#{name} is #{value.length} characters, over Apple's limit of #{limit}." if limit && value.length > limit
  value
end

Spaceship::ConnectAPI.login(ENV["APPLE_EMAIL"], nil, use_portal: false, use_tunes: true)

app = Spaceship::ConnectAPI::App.find(BUNDLE_ID)
abort "No app on team #{ITC_TEAM_ID} for #{BUNDLE_ID}. Create it with produce first." unless app
puts "#{app.name} (#{app.id})"

info = app.fetch_edit_app_info
abort "No editable app info; the listing may already be in review." unless info

localization = info.get_app_info_localizations.find { |l| l.locale == LOCALE }
localization ||= info.create_app_info_localization(attributes: { locale: LOCALE })
# The name is the one field a person is likely to change by hand in App Store Connect, and losing
# somebody's naming decision to a stale text file is not a good trade. So divergence is reported
# and the live value kept, rather than overwritten.
attributes = { subtitle: field("subtitle"), privacyPolicyUrl: field("privacy_url") }
wanted_name = field("name")
if wanted_name && localization.name && wanted_name != localization.name
  puts "  keeping the name set in App Store Connect (#{localization.name.inspect});"
  puts "  #{DIR}/#{LOCALE}/name.txt says #{wanted_name.inspect}. Update the file to match, or pass"
  puts "  FORCE_NAME=1 to make the file win."
  attributes[:name] = wanted_name if ENV["FORCE_NAME"]
else
  attributes[:name] = wanted_name
end

localization.update(attributes: attributes)
puts "  subtitle and privacy policy set"

category = field("primary_category", localized: false)
if category
  info.update_categories(category_id_map: { primary_category_id: category })
  puts "  primary category set to #{category}"
end

version = app.get_edit_app_store_version(platform: Spaceship::ConnectAPI::Platform::MAC_OS)
abort "No editable macOS version to write to." unless version

# The store version has to match the CFBundleShortVersionString of the build that will be uploaded
# against it, and that comes from tauri.conf.json like every other version in the repo. Left alone,
# a record created by `produce` sits at 1.0 and rejects the first build with a version mismatch.
app_version = JSON.parse(File.read("src-tauri/tauri.conf.json"))["version"]
if version.version_string != app_version
  version.update(attributes: { versionString: app_version })
  puts "  version corrected from #{version.version_string} to #{app_version}"
  version = app.get_edit_app_store_version(platform: Spaceship::ConnectAPI::Platform::MAC_OS)
end

version_localization = version.get_app_store_version_localizations.find { |l| l.locale == LOCALE }
version_localization ||= version.create_app_store_version_localization(attributes: { locale: LOCALE })
attributes = {
  description: field("description"),
  keywords: field("keywords"),
  promotionalText: field("promotional_text"),
  supportUrl: field("support_url"),
  marketingUrl: field("marketing_url"),
}

# Release notes describe what changed since the last release, so Apple refuses them on a first
# version and there is nothing truthful to put there anyway.
if app.get_live_app_store_version(platform: Spaceship::ConnectAPI::Platform::MAC_OS)
  attributes[:whatsNew] = field("release_notes")
else
  puts "  skipping release notes: nothing has shipped yet for them to be relative to"
end

version_localization.update(attributes: attributes)
puts "  description, keywords and links set on version #{version.version_string}"

copyright = field("copyright", localized: false)
version.update(attributes: { copyright: copyright }) if copyright
puts "  copyright set to #{copyright}" if copyright

puts
puts "Screenshots are not set here. They are required to submit for review, but not to upload a"
puts "build or to run either tier of TestFlight."
