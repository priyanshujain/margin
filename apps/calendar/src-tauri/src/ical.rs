// iCalendar (RFC 5545) read into Calendar's neutral events, and edits written back into a resource
// without disturbing what Calendar does not model (alarms, attendees, other overrides). Enough
// of the format for files people export, the feeds they subscribe to and what a CalDAV server
// stores: VEVENT with its times, rules, exceptions and people. Everything else is skipped.

use chrono::{DateTime, Duration, LocalResult, NaiveDate, NaiveDateTime, SecondsFormat, TimeZone};
use rrule::Tz;

use crate::dto::Attendee;
use crate::provider::{EventWrite, RemoteEvent, When};

#[derive(Debug, Default)]
pub struct Feed {
    pub name: Option<String>,
    pub events: Vec<RemoteEvent>,
}

struct Property {
    name: String,
    params: Vec<(String, String)>,
    value: String,
}

impl Property {
    fn param(&self, key: &str) -> Option<&str> {
        self.params
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(key))
            .map(|(_, value)| value.as_str())
    }
}

/// Continuation lines start with a space or a tab and join the line before them.
fn unfold(text: &str) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for raw in text.split('\n') {
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        if let Some(rest) = line.strip_prefix(' ').or_else(|| line.strip_prefix('\t'))
            && let Some(last) = lines.last_mut()
        {
            last.push_str(rest);
            continue;
        }
        if !line.is_empty() {
            lines.push(line.to_string());
        }
    }
    lines
}

/// `NAME;PARAM=a;PARAM="b:c":value`, where a colon inside quotes is not the separator.
fn property(line: &str) -> Option<Property> {
    let mut quoted = false;
    let mut split = None;
    for (at, ch) in line.char_indices() {
        match ch {
            '"' => quoted = !quoted,
            ':' if !quoted => {
                split = Some(at);
                break;
            }
            _ => {}
        }
    }
    let at = split?;
    let (head, value) = (&line[..at], &line[at + 1..]);
    let mut parts = head.split(';');
    let name = parts.next()?.trim().to_ascii_uppercase();
    let params = parts
        .filter_map(|part| {
            let (key, value) = part.split_once('=')?;
            Some((key.trim().to_ascii_uppercase(), value.trim_matches('"').to_string()))
        })
        .collect();
    Some(Property {
        name,
        params,
        value: value.to_string(),
    })
}

fn unescape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        match chars.next() {
            Some('n') | Some('N') => out.push('\n'),
            Some(other) => out.push(other),
            None => {}
        }
    }
    out
}

fn escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace(';', "\\;")
        .replace(',', "\\,")
        .replace('\n', "\\n")
}

fn known_zone(name: Option<&str>) -> Option<chrono_tz::Tz> {
    name.and_then(|name| name.trim().parse::<chrono_tz::Tz>().ok())
}

fn resolve(zone: Tz, naive: NaiveDateTime) -> DateTime<Tz> {
    match zone.from_local_datetime(&naive) {
        LocalResult::Single(at) | LocalResult::Ambiguous(at, _) => at,
        LocalResult::None => zone
            .from_local_datetime(&(naive + Duration::hours(1)))
            .earliest()
            .map(|at| at - Duration::hours(1))
            .unwrap_or_else(|| Tz::UTC.from_utc_datetime(&naive).with_timezone(&zone)),
    }
}

/// A DATE or DATE-TIME property as Calendar stores it. UTC stays UTC, a known zone is kept as the
/// event's zone, and a floating time or an unknown zone is read on this machine's clock.
fn when(prop: &Property) -> Option<When> {
    let value = prop.value.trim();
    if prop.param("VALUE").is_some_and(|v| v.eq_ignore_ascii_case("DATE")) || value.len() == 8 {
        let date = NaiveDate::parse_from_str(value, "%Y%m%d").ok()?;
        return Some(When {
            at: date.format("%Y-%m-%d").to_string(),
            zone: None,
        });
    }
    if let Some(utc) = value.strip_suffix('Z') {
        let naive = NaiveDateTime::parse_from_str(utc, "%Y%m%dT%H%M%S").ok()?;
        return Some(When {
            at: Tz::UTC
                .from_utc_datetime(&naive)
                .to_rfc3339_opts(SecondsFormat::Secs, true),
            zone: None,
        });
    }
    let naive = NaiveDateTime::parse_from_str(value, "%Y%m%dT%H%M%S").ok()?;
    let zone = known_zone(prop.param("TZID"));
    let at = resolve(zone.map(Tz::Tz).unwrap_or(Tz::LOCAL), naive);
    Some(When {
        at: at.to_rfc3339_opts(SecondsFormat::Secs, false),
        zone: zone.map(|zone| zone.name().to_string()),
    })
}

/// `P1D`, `PT1H30M`, `-P1W`. Months and years are not durations in this format.
fn duration(value: &str) -> Option<Duration> {
    let (sign, rest) = match value.strip_prefix('-') {
        Some(rest) => (-1, rest),
        None => (1, value.strip_prefix('+').unwrap_or(value)),
    };
    let rest = rest.strip_prefix('P')?;
    let mut total = Duration::zero();
    let mut number = String::new();
    let mut timed = false;
    for ch in rest.chars() {
        match ch {
            'T' => timed = true,
            '0'..='9' => number.push(ch),
            unit => {
                let n: i64 = number.parse().ok()?;
                number.clear();
                total += match (unit, timed) {
                    ('W', _) => Duration::weeks(n),
                    ('D', _) => Duration::days(n),
                    ('H', true) => Duration::hours(n),
                    ('M', true) => Duration::minutes(n),
                    ('S', true) => Duration::seconds(n),
                    _ => return None,
                };
            }
        }
    }
    Some(total * sign)
}

fn shifted(start: &When, by: Duration) -> Option<When> {
    if start.all_day() {
        let date = NaiveDate::parse_from_str(&start.at, "%Y-%m-%d").ok()?;
        return Some(When {
            at: (date + by).format("%Y-%m-%d").to_string(),
            zone: None,
        });
    }
    let at = DateTime::parse_from_rfc3339(&start.at).ok()?;
    Some(When {
        at: (at + by).to_rfc3339_opts(SecondsFormat::Secs, start.at.ends_with('Z')),
        zone: start.zone.clone(),
    })
}

fn person(prop: &Property, organizer: bool) -> Option<Attendee> {
    let address = prop.value.trim();
    let email = address
        .strip_prefix("mailto:")
        .or_else(|| address.strip_prefix("MAILTO:"))
        .unwrap_or(address)
        .to_string();
    if email.is_empty() {
        return None;
    }
    let response_status = match prop.param("PARTSTAT").map(str::to_ascii_uppercase).as_deref() {
        Some("ACCEPTED") => "accepted",
        Some("DECLINED") => "declined",
        Some("TENTATIVE") => "tentative",
        _ => "needsAction",
    };
    Some(Attendee {
        email,
        display_name: prop.param("CN").map(str::to_string),
        response_status: response_status.to_string(),
        organizer,
        is_self: false,
        optional: prop.param("ROLE").is_some_and(|r| r.eq_ignore_ascii_case("OPT-PARTICIPANT")),
    })
}

/// An exception shares its series' UID, so it is stored under the UID and the start it replaces.
pub fn exception_id(uid: &str, original_start: &str) -> String {
    format!("{uid}::{original_start}")
}

fn event(props: &[Property]) -> Option<RemoteEvent> {
    let find = |name: &str| props.iter().find(|p| p.name == name);
    let text = |name: &str| find(name).map(|p| unescape(&p.value)).filter(|v| !v.is_empty());

    let uid = text("UID")?;
    let start = find("DTSTART").and_then(when)?;
    let end = find("DTEND").and_then(when).or_else(|| {
        let by = find("DURATION")
            .and_then(|p| duration(p.value.trim()))
            .unwrap_or_else(|| if start.all_day() { Duration::days(1) } else { Duration::zero() });
        shifted(&start, by)
    });
    let original_start = find("RECURRENCE-ID").and_then(when).map(|w| w.at);
    let recurrence = props
        .iter()
        .filter(|p| matches!(p.name.as_str(), "RRULE" | "RDATE" | "EXDATE"))
        .map(|p| {
            let params: String = p.params.iter().map(|(k, v)| format!(";{k}={v}")).collect();
            format!("{}{params}:{}", p.name, p.value.trim())
        })
        .collect();

    let mut attendees: Vec<Attendee> = props
        .iter()
        .filter(|p| p.name == "ATTENDEE")
        .filter_map(|p| person(p, false))
        .collect();
    if let Some(organizer) = find("ORGANIZER").and_then(|p| person(p, true)) {
        match attendees.iter_mut().find(|a| a.email.eq_ignore_ascii_case(&organizer.email)) {
            Some(known) => known.organizer = true,
            None => attendees.push(organizer),
        }
    }

    Some(RemoteEvent {
        id: match &original_start {
            Some(original) => exception_id(&uid, original),
            None => uid.clone(),
        },
        version: None,
        status: text("STATUS")
            .map(|s| s.to_ascii_lowercase())
            .unwrap_or_else(|| "confirmed".to_string()),
        summary: text("SUMMARY").unwrap_or_default(),
        description: text("DESCRIPTION"),
        location: text("LOCATION"),
        start: Some(start),
        end,
        recurrence,
        series_id: original_start.is_some().then_some(uid),
        original_start,
        attendees,
        conference: None,
        updated: text("LAST-MODIFIED").or_else(|| text("DTSTAMP")),
        color: None,
        remote_ref: None,
    })
}

pub fn parse(text: &str) -> Result<Feed, String> {
    let lines = unfold(text);
    if !lines
        .first()
        .is_some_and(|l| l.trim().eq_ignore_ascii_case("BEGIN:VCALENDAR"))
    {
        return Err("This is not an iCalendar file.".to_string());
    }
    let mut feed = Feed::default();
    let mut depth: Vec<String> = Vec::new();
    let mut current: Vec<Property> = Vec::new();
    for line in &lines {
        let Some(prop) = property(line) else { continue };
        match prop.name.as_str() {
            "BEGIN" => {
                depth.push(prop.value.trim().to_ascii_uppercase());
                if depth.last().map(String::as_str) == Some("VEVENT") {
                    current.clear();
                }
            }
            "END" => {
                if depth.pop().as_deref() == Some("VEVENT")
                    && let Some(found) = event(&current)
                {
                    feed.events.push(found);
                }
            }
            "X-WR-CALNAME" if depth.len() == 1 => {
                feed.name = Some(unescape(&prop.value)).filter(|n| !n.trim().is_empty());
            }
            _ if depth.last().map(String::as_str) == Some("VEVENT") => current.push(prop),
            _ => {}
        }
    }
    Ok(feed)
}

fn ical_when(name: &str, value: &When) -> Option<String> {
    if value.all_day() {
        let date = NaiveDate::parse_from_str(&value.at, "%Y-%m-%d").ok()?;
        return Some(format!("{name};VALUE=DATE:{}", date.format("%Y%m%d")));
    }
    let at = DateTime::parse_from_rfc3339(&value.at).ok()?;
    match known_zone(value.zone.as_deref()) {
        Some(zone) => Some(format!(
            "{name};TZID={}:{}",
            zone.name(),
            at.with_timezone(&zone).format("%Y%m%dT%H%M%S")
        )),
        None => Some(format!(
            "{name}:{}",
            at.with_timezone(&chrono::Utc).format("%Y%m%dT%H%M%SZ")
        )),
    }
}

/// Lines past 75 octets are folded, at a character boundary.
fn fold(line: &str) -> String {
    let mut out = String::new();
    let mut width = 0;
    for ch in line.chars() {
        let size = ch.len_utf8();
        if width + size > 75 {
            out.push_str("\r\n ");
            width = 1;
        }
        out.push(ch);
        width += size;
    }
    out
}

fn join(lines: &[String]) -> String {
    lines.iter().map(|l| fold(l)).collect::<Vec<_>>().join("\r\n") + "\r\n"
}

fn name_of(line: &str) -> String {
    property(line).map(|p| p.name).unwrap_or_default()
}

/// The lines a write puts into a VEVENT, for the fields it carries.
fn write_lines(write: &EventWrite) -> Result<Vec<String>, String> {
    let mut lines = Vec::new();
    if let Some(start) = &write.start {
        lines.push(ical_when("DTSTART", start).ok_or("the start is not a time")?);
    }
    if let Some(end) = &write.end {
        lines.push(ical_when("DTEND", end).ok_or("the end is not a time")?);
    }
    if let Some(summary) = &write.summary {
        lines.push(format!("SUMMARY:{}", escape(summary)));
    }
    for (name, value) in [("DESCRIPTION", &write.description), ("LOCATION", &write.location)] {
        if let Some(value) = value.as_deref().filter(|v| !v.is_empty()) {
            lines.push(format!("{name}:{}", escape(value)));
        }
    }
    for rule in write.recurrence.iter().flatten() {
        lines.push(rule.clone());
    }
    Ok(lines)
}

/// The property names a write replaces. Clearing a text field removes it and adds nothing.
fn replaced_by(write: &EventWrite) -> Vec<&'static str> {
    let mut names = Vec::new();
    if write.start.is_some() {
        names.push("DTSTART");
    }
    if write.end.is_some() || write.start.is_some() {
        names.extend(["DTEND", "DURATION"]);
    }
    if write.summary.is_some() {
        names.push("SUMMARY");
    }
    if write.description.is_some() {
        names.push("DESCRIPTION");
    }
    if write.location.is_some() {
        names.push("LOCATION");
    }
    if write.recurrence.is_some() {
        names.extend(["RRULE", "RDATE", "EXDATE"]);
    }
    names
}

/// A brand new resource holding one event, for a CalDAV create.
pub fn new_resource(uid: &str, write: &EventWrite, stamp: &str) -> Result<String, String> {
    if write.start.is_none() || write.end.is_none() {
        return Err("a new event needs a start and an end".to_string());
    }
    let mut lines = vec![
        "BEGIN:VCALENDAR".to_string(),
        "VERSION:2.0".to_string(),
        "PRODID:-//Margin//Margin Calendar//EN".to_string(),
        "BEGIN:VEVENT".to_string(),
        format!("UID:{uid}"),
        format!("DTSTAMP:{stamp}"),
    ];
    lines.extend(write_lines(write)?);
    lines.push("END:VEVENT".to_string());
    lines.push("END:VCALENDAR".to_string());
    Ok(join(&lines))
}

/// One VEVENT block in a resource: where it sits and which occurrence it overrides, if any.
struct Block {
    begin: usize,
    end: usize,
    recurrence_id: Option<String>,
}

fn blocks(lines: &[String]) -> Vec<Block> {
    let mut found = Vec::new();
    let mut open: Option<(usize, Option<String>)> = None;
    let mut depth = 0;
    for (at, line) in lines.iter().enumerate() {
        let Some(prop) = property(line) else { continue };
        match prop.name.as_str() {
            "BEGIN" => {
                depth += 1;
                if prop.value.eq_ignore_ascii_case("VEVENT") {
                    open = Some((at, None));
                }
            }
            "END" => {
                depth -= 1;
                if prop.value.eq_ignore_ascii_case("VEVENT")
                    && let Some((begin, recurrence_id)) = open.take()
                {
                    found.push(Block { begin, end: at, recurrence_id });
                }
            }
            "RECURRENCE-ID" if depth == 2 => {
                if let Some((_, slot)) = open.as_mut() {
                    *slot = when(&prop).map(|w| w.at);
                }
            }
            _ => {}
        }
    }
    found
}

/// Two stored times name the same occurrence: equal dates, or equal instants whatever the offset.
fn same_time(a: &str, b: &str) -> bool {
    if a.len() == 10 || b.len() == 10 {
        return a == b;
    }
    match (DateTime::parse_from_rfc3339(a), DateTime::parse_from_rfc3339(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

/// Each line inside a VEVENT, with whether it is the event's own rather than a nested component's,
/// such as a VALARM, whose DESCRIPTION and TRIGGER must never be mistaken for the event's.
fn own_lines(lines: &[String]) -> Vec<(&String, bool)> {
    let mut depth = 0usize;
    lines
        .iter()
        .map(|line| {
            let name = name_of(line);
            if name == "BEGIN" {
                depth += 1;
                (line, false)
            } else if name == "END" {
                depth = depth.saturating_sub(1);
                (line, false)
            } else {
                (line, depth == 0)
            }
        })
        .collect()
}

fn patch_block(lines: &mut Vec<String>, block: &Block, write: &EventWrite, stamp: &str) -> Result<(), String> {
    let drop = replaced_by(write);
    let mut body: Vec<String> = own_lines(&lines[block.begin + 1..block.end])
        .into_iter()
        .filter(|(line, own)| {
            let name = name_of(line);
            !own || (!drop.contains(&name.as_str()) && name != "DTSTAMP")
        })
        .map(|(line, _)| line.clone())
        .collect();
    let mut added = vec![format!("DTSTAMP:{stamp}")];
    added.extend(write_lines(write)?);
    let at = first_nested(&body);
    body.splice(at..at, added);
    lines.splice(block.begin + 1..block.end, body);
    Ok(())
}

/// Where a new property goes: before the first nested component, since an event's properties come
/// before its alarms.
fn first_nested(body: &[String]) -> usize {
    body.iter()
        .position(|line| name_of(line) == "BEGIN")
        .unwrap_or(body.len())
}

fn master_facts(lines: &[String], master: &Block) -> (Option<When>, Option<When>, Option<String>) {
    let mut start = None;
    let mut end = None;
    let mut uid = None;
    for (line, own) in own_lines(&lines[master.begin + 1..master.end]) {
        if !own {
            continue;
        }
        let Some(prop) = property(line) else { continue };
        match prop.name.as_str() {
            "DTSTART" => start = when(&prop),
            "DTEND" => end = when(&prop),
            "DURATION" => {
                if let (Some(s), Some(by)) = (start.as_ref(), duration(prop.value.trim())) {
                    end = shifted(s, by);
                }
            }
            "UID" => uid = Some(prop.value.trim().to_string()),
            _ => {}
        }
    }
    (start, end, uid)
}

/// Writes `write` into the event `original_start` names: the master when it is None, otherwise the
/// override for that occurrence, which is made from the master when the resource has none yet.
pub fn apply(text: &str, original_start: Option<&str>, write: &EventWrite, stamp: &str) -> Result<String, String> {
    let mut lines = unfold(text);
    let found = blocks(&lines);
    let master = found
        .iter()
        .find(|b| b.recurrence_id.is_none())
        .ok_or("the resource has no event in it")?;
    let Some(original) = original_start else {
        patch_block(&mut lines, master, write, stamp)?;
        return Ok(join(&lines));
    };
    if let Some(over) = found
        .iter()
        .find(|b| b.recurrence_id.as_deref().is_some_and(|r| same_time(r, original)))
    {
        patch_block(&mut lines, over, write, stamp)?;
        return Ok(join(&lines));
    }

    let (start, end, uid) = master_facts(&lines, master);
    let start = start.ok_or("the series has no start")?;
    let span = match (&start, &end) {
        (s, Some(e)) if s.all_day() => {
            let day = |v: &str| NaiveDate::parse_from_str(v, "%Y-%m-%d").ok();
            day(&e.at).zip(day(&s.at)).map(|(e, s)| e - s).unwrap_or(Duration::days(1))
        }
        (s, Some(e)) => match (DateTime::parse_from_rfc3339(&s.at), DateTime::parse_from_rfc3339(&e.at)) {
            (Ok(s), Ok(e)) => e - s,
            _ => Duration::zero(),
        },
        _ => Duration::zero(),
    };
    let occurrence = When { at: original.to_string(), zone: start.zone.clone() };
    let mut copy = EventWrite {
        start: Some(occurrence.clone()),
        end: shifted(&occurrence, span),
        ..Default::default()
    };
    copy.overlay(write);
    copy.recurrence = None;

    let inherited: Vec<String> = own_lines(&lines[master.begin + 1..master.end])
        .into_iter()
        .filter(|(line, own)| {
            *own
                && matches!(name_of(line).as_str(), "SUMMARY" | "DESCRIPTION" | "LOCATION" | "ORGANIZER" | "ATTENDEE")
                && !replaced_by(&copy).contains(&name_of(line).as_str())
        })
        .map(|(line, _)| line.clone())
        .collect();
    let mut block = vec![
        "BEGIN:VEVENT".to_string(),
        format!("UID:{}", uid.ok_or("the series has no UID")?),
        format!("DTSTAMP:{stamp}"),
        ical_when("RECURRENCE-ID", &occurrence).ok_or("the occurrence is not a time")?,
    ];
    block.extend(inherited);
    block.extend(write_lines(&copy)?);
    block.push("END:VEVENT".to_string());
    let at = lines
        .iter()
        .rposition(|l| l.trim().eq_ignore_ascii_case("END:VCALENDAR"))
        .ok_or("the resource does not end")?;
    lines.splice(at..at, block);
    Ok(join(&lines))
}

/// Drops one occurrence: an EXDATE on the master, and the override for it if there is one.
pub fn exclude(text: &str, original_start: &str, stamp: &str) -> Result<String, String> {
    let mut lines = unfold(text);
    let found = blocks(&lines);
    if let Some(over) = found
        .iter()
        .find(|b| b.recurrence_id.as_deref().is_some_and(|r| same_time(r, original_start)))
    {
        lines.drain(over.begin..=over.end);
    }
    let found = blocks(&lines);
    let master = found
        .iter()
        .find(|b| b.recurrence_id.is_none())
        .ok_or("the resource has no event in it")?;
    let (start, _, _) = master_facts(&lines, master);
    let zone = start.and_then(|s| s.zone);
    let line = ical_when("EXDATE", &When { at: original_start.to_string(), zone })
        .ok_or("the occurrence is not a time")?;
    let stamp_at = own_lines(&lines[master.begin + 1..master.end])
        .iter()
        .position(|(l, own)| *own && name_of(l) == "DTSTAMP")
        .map(|at| master.begin + 1 + at);
    if let Some(at) = stamp_at {
        lines[at] = format!("DTSTAMP:{stamp}");
    }
    let at = master.begin + 1 + first_nested(&lines[master.begin + 1..master.end]);
    lines.insert(at, line);
    Ok(join(&lines))
}
