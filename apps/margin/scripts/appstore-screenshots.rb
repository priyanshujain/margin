#!/usr/bin/env ruby
# Upload the rendered frames in appstore/screenshots to the App Store listing.
#
#   APPLE_EMAIL=you@example.com ruby scripts/appstore-screenshots.rb
#
# The frames are rendered by the HTML under appstore/screenshots/src, so this only ever moves
# finished PNGs. Rerunning replaces what is there rather than appending, because a listing that
# quietly accumulated ten frames across five runs would be worse than one that is simply current.
begin
  require "spaceship"
rescue LoadError
  libexec = Dir["/opt/homebrew/Cellar/fastlane/*/libexec", "/usr/local/Cellar/fastlane/*/libexec"].max
  abort "spaceship is not installed. `brew install fastlane` and rerun." unless libexec
  ENV["GEM_PATH"] = [libexec, ENV["GEM_PATH"]].compact.join(":")
  Gem.clear_paths
  require "spaceship"
end

require "shellwords"

DIR = ENV.fetch("SCREENSHOT_DIR", "appstore/screenshots")
LOCALE = "en-US"
BUNDLE_ID = ENV.fetch("BUNDLE_ID", "studio.margin.app")
ENV["FASTLANE_ITC_TEAM_ID"] = ENV.fetch("FASTLANE_ITC_TEAM_ID", "129377371")

# The only size App Store Connect takes for a Mac app, out of the four it documents, that this
# pipeline renders. Anything else is a mistake worth stopping for.
EXPECTED = [2560, 1600].freeze

frames = Dir[File.join(DIR, "frame-*.png")].sort
abort "No frames in #{DIR}." if frames.empty?

frames.each do |path|
  dimensions = `sips -g pixelWidth -g pixelHeight #{path.shellescape} 2>/dev/null`
    .scan(/pixel(?:Width|Height):\s*(\d+)/).flatten.map(&:to_i)
  next if dimensions == EXPECTED

  abort "#{path} is #{dimensions.join('x')}, and Apple wants #{EXPECTED.join('x')}."
end
puts "#{frames.size} frames, all #{EXPECTED.join('x')}"

Spaceship::ConnectAPI.login(ENV["APPLE_EMAIL"], nil, use_portal: false, use_tunes: true)

app = Spaceship::ConnectAPI::App.find(BUNDLE_ID)
abort "No app for #{BUNDLE_ID}." unless app
puts "#{app.name} (#{app.id})"

version = app.get_edit_app_store_version(platform: Spaceship::ConnectAPI::Platform::MAC_OS)
abort "No editable macOS version." unless version

localization = version.get_app_store_version_localizations.find { |l| l.locale == LOCALE }
abort "No #{LOCALE} localization; run appstore-listing.rb first." unless localization

display_type = Spaceship::ConnectAPI::AppScreenshotSet::DisplayType::APP_DESKTOP
set = localization.get_app_screenshot_sets.find { |s| s.screenshot_display_type == display_type }

if set
  set.app_screenshots.each(&:delete!)
  puts "  cleared #{set.app_screenshots.size} existing screenshots"
else
  set = localization.create_app_screenshot_set(attributes: { screenshotDisplayType: display_type })
  puts "  created the desktop screenshot set"
end

frames.each_with_index do |path, index|
  set.upload_screenshot(path: path, position: index)
  puts "  uploaded #{File.basename(path)}"
end

puts
puts "Apple processes each image before it counts as attached; give it a minute before checking."
