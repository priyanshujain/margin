/// Built here because reqwest's own query builder sits behind a feature this build does not carry.
pub fn query_string(params: &[(&str, &str)]) -> String {
    let mut serializer = url::form_urlencoded::Serializer::new(String::new());
    for (key, value) in params {
        serializer.append_pair(key, value);
    }
    serializer.finish()
}

pub fn url_with(base: &str, params: &[(&str, &str)]) -> String {
    if params.is_empty() {
        base.to_string()
    } else {
        format!("{base}?{}", query_string(params))
    }
}

pub fn path_segment(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_percent_encoded_into_the_path() {
        assert_eq!(path_segment("18f9a2b3c4d5e6f7"), "18f9a2b3c4d5e6f7");
        assert_eq!(path_segment("r-123/456"), "r-123%2F456");
    }
}
