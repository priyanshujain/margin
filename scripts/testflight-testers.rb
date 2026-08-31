#!/usr/bin/env ruby
# Invite people to a TestFlight group.
#
#   APPLE_EMAIL=you@example.com ruby scripts/testflight-testers.rb someone@example.com ...
#
# Addresses come from the command line rather than a file in the repo, because a list of testers is
# a list of people's email addresses and this repository is public.
#
# GROUP defaults to the external group, since the internal one only accepts people who already have
# an App Store Connect account on the team, which is a far bigger thing to hand out than a build.
begin
  require "spaceship"
rescue LoadError
  libexec = Dir["/opt/homebrew/Cellar/fastlane/*/libexec", "/usr/local/Cellar/fastlane/*/libexec"].max
  abort "spaceship is not installed. `brew install fastlane` and rerun." unless libexec
  ENV["GEM_PATH"] = [libexec, ENV["GEM_PATH"]].compact.join(":")
  Gem.clear_paths
  require "spaceship"
end

BUNDLE_ID = ENV.fetch("BUNDLE_ID", "studio.margin.app")
GROUP = ENV.fetch("GROUP", "Public Beta")
ENV["FASTLANE_ITC_TEAM_ID"] = ENV.fetch("FASTLANE_ITC_TEAM_ID", "129377371")

emails = ARGV.reject { |a| a.start_with?("-") }
abort "Usage: ruby scripts/testflight-testers.rb someone@example.com [more@example.com ...]" if emails.empty?

Spaceship::ConnectAPI.login(ENV["APPLE_EMAIL"], nil, use_portal: false, use_tunes: true)

app = Spaceship::ConnectAPI::App.find(BUNDLE_ID)
abort "No app for #{BUNDLE_ID}." unless app

group = app.get_beta_groups.find { |g| g.name == GROUP }
abort "No group named #{GROUP.inspect}." unless group
puts "#{app.name}: #{group.name} (#{group.is_internal_group ? 'internal' : 'external'})"

existing = Spaceship::ConnectAPI::BetaTester
  .all(filter: { betaGroups: group.id })
  .map { |t| t.email.to_s.downcase }

emails.each do |email|
  if existing.include?(email.downcase)
    puts "  #{email} is already in this group"
    next
  end

  Spaceship::ConnectAPI.post_bulk_beta_tester_assignments(
    beta_group_id: group.id,
    beta_testers: [{ email: email }],
  )
  puts "  invited #{email}"
end

puts
if group.is_internal_group
  puts "Internal testers can install as soon as a build finishes processing."
else
  puts "External testers get the invitation once beta app review passes."
  puts "Anyone can also join through #{group.public_link}" if group.public_link
end
