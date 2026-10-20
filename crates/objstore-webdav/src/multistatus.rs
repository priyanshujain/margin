use quick_xml::Reader;
use quick_xml::events::Event;

#[derive(Debug, Default)]
pub(crate) struct Entry {
    pub href: String,
    pub collection: bool,
    pub etag: String,
    pub size: u64,
    pub modified: Option<String>,
}

pub(crate) fn parse(body: &str) -> Result<Vec<Entry>, String> {
    let mut reader = Reader::from_str(body);
    let mut entries = Vec::new();
    let mut current: Option<Entry> = None;
    let mut ok = true;
    let mut element = Vec::<u8>::new();
    loop {
        match reader
            .read_event()
            .map_err(|e| format!("unreadable PROPFIND answer: {e}"))?
        {
            Event::Start(start) => {
                let name = start.local_name().as_ref().to_vec();
                match name.as_slice() {
                    b"response" => current = Some(Entry::default()),
                    b"propstat" => ok = true,
                    b"collection" => mark_collection(&mut current, ok),
                    _ => {}
                }
                element = name;
            }
            Event::Empty(empty) => {
                if empty.local_name().as_ref() == b"collection" {
                    mark_collection(&mut current, ok);
                }
            }
            Event::Text(text) => {
                let value = text.decode().map_err(|e| e.to_string())?.into_owned();
                let value = quick_xml::escape::unescape(&value)
                    .map_err(|e| e.to_string())?
                    .into_owned();
                apply(&mut current, &element, &value, &mut ok);
            }
            Event::GeneralRef(reference) => {
                let name = reference.decode().map_err(|e| e.to_string())?;
                let value = quick_xml::escape::unescape(&format!("&{name};"))
                    .map_err(|e| e.to_string())?
                    .into_owned();
                apply(&mut current, &element, &value, &mut ok);
            }
            Event::End(end) => {
                if end.local_name().as_ref() == b"response"
                    && let Some(entry) = current.take()
                {
                    entries.push(entry);
                }
                element.clear();
            }
            Event::Eof => return Ok(entries),
            _ => {}
        }
    }
}

fn mark_collection(current: &mut Option<Entry>, ok: bool) {
    if let Some(entry) = current
        && ok
    {
        entry.collection = true;
    }
}

fn apply(current: &mut Option<Entry>, element: &[u8], value: &str, ok: &mut bool) {
    let Some(entry) = current else {
        return;
    };
    match element {
        b"href" => entry.href.push_str(value),
        b"status" => *ok = value.contains(" 200 "),
        b"getetag" if *ok => entry.etag.push_str(value),
        b"getcontentlength" if *ok => entry.size = value.trim().parse().unwrap_or_default(),
        b"getlastmodified" if *ok => entry.modified = Some(value.to_string()),
        _ => {}
    }
}
