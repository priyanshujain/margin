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

# An internal tester has to be a user on the App Store Connect account first, which is a far larger
# grant than a build: it is access to the account, not to an app. So the invitation is narrowed as
# far as the API allows, to this one app and with provisioning refused, and the person still has to
# accept it before they can be put in the group.
if group.is_internal_group
  known = Spaceship::ConnectAPI::User.all.map { |u| u.email.to_s.downcase }
  invited = Spaceship::ConnectAPI::UserInvitation.all.map { |i| i.email.to_s.downcase }

  emails.each do |email|
    next if known.include?(email.downcase)

    if invited.include?(email.downcase)
      puts "  #{email} has an invitation waiting to be accepted"
      next
    end

    local = email.split("@").first
    Spaceship::ConnectAPI::UserInvitation.create(
      email: email,
      first_name: ENV.fetch("FIRST_NAME", local),
      last_name: ENV.fetch("LAST_NAME", "Tester"),
      roles: [Spaceship::ConnectAPI::User::UserRole::DEVELOPER],
      provisioning_allowed: false,
      all_apps_visible: false,
      visible_app_ids: [app.id],
    )
    puts "  invited #{email} to App Store Connect, limited to this app, no provisioning access"
    invited << email.downcase
  end
end

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

  # The bulk endpoint reports success for an address it then quietly declines to add, which is what
  # happens on an internal group when the person has not accepted their account invitation yet. So
  # the group is read back rather than trusted.
  landed = Spaceship::ConnectAPI::BetaTester
    .all(filter: { betaGroups: group.id })
    .any? { |t| t.email.to_s.casecmp?(email) }

  if landed
    puts "  added #{email}"
  else
    puts "  #{email} was not added. An internal tester has to accept the App Store Connect"
    puts "  invitation first; rerun this once they have."
  end
end

puts
if group.is_internal_group
  puts "Internal testers can install as soon as a build finishes processing."
else
  puts "External testers get the invitation once beta app review passes."
  puts "Anyone can also join through #{group.public_link}" if group.public_link
end
