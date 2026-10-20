use margin_crypt::{CryptError, Key, Phrase, RootKey, mail_backup_key, open, seal};

fn unhex(text: &str) -> Vec<u8> {
    (0..text.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(&text[at..at + 2], 16).unwrap())
        .collect()
}

#[test]
fn opens_a_backup_written_by_margin_mail() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/mail-backup-v1.json")).unwrap();
    let phrase = Phrase::parse(fixture["phrase"].as_str().unwrap()).unwrap();
    let key = mail_backup_key(&phrase).unwrap();
    let name = fixture["name"].as_str().unwrap();
    let sealed = unhex(fixture["sealed_hex"].as_str().unwrap());
    assert_eq!(
        open(&key, name, &sealed).unwrap(),
        unhex(fixture["plain_hex"].as_str().unwrap())
    );
    assert_eq!(
        open(&key, "other/name.seg", &sealed),
        Err(CryptError::Decrypt)
    );

    let typed = fixture["phrase"]
        .as_str()
        .unwrap()
        .to_uppercase()
        .replace(' ', "  \n");
    assert_eq!(
        mail_backup_key(&Phrase::parse(&typed).unwrap()).unwrap(),
        key
    );
}

#[test]
fn round_trip_and_failures() {
    let key = Key::from_bytes([7; 32]);
    let plain = b"a note about lunch".to_vec();
    let sealed = seal(&key, "notes/1", &plain).unwrap();
    assert!(!sealed.windows(plain.len()).any(|w| w == plain));
    assert_eq!(open(&key, "notes/1", &sealed).unwrap(), plain);
    assert_ne!(seal(&key, "notes/1", &plain).unwrap(), sealed);

    assert_eq!(
        open(&Key::from_bytes([8; 32]), "notes/1", &sealed),
        Err(CryptError::Decrypt)
    );
    assert_eq!(open(&key, "notes/2", &sealed), Err(CryptError::Decrypt));
    let mut flipped = sealed.clone();
    *flipped.last_mut().unwrap() ^= 1;
    assert_eq!(open(&key, "notes/1", &flipped), Err(CryptError::Decrypt));
    assert_eq!(
        open(&key, "notes/1", &sealed[..40]),
        Err(CryptError::Truncated)
    );
    assert_eq!(
        open(&key, "notes/1", b"nope, not sealed"),
        Err(CryptError::NotSealed)
    );
    assert_eq!(
        open(&key, "e", &seal(&key, "e", b"").unwrap()).unwrap(),
        b""
    );
}

#[test]
fn phrase_derives_stable_distinct_app_keys() {
    let phrase = Phrase::generate().unwrap();
    assert_eq!(phrase.as_str().split(' ').count(), 24);
    assert_eq!(format!("{phrase:?}"), "Phrase(not shown)");
    let root = RootKey::from_phrase(&phrase).unwrap();
    assert_eq!(
        RootKey::from_phrase(&Phrase::parse(phrase.as_str()).unwrap()).unwrap(),
        root
    );
    assert_ne!(root.app_key("mail"), root.app_key("docs"));
    assert_eq!(root.app_key("mail"), root.app_key("mail"));
    assert_ne!(
        root.as_bytes(),
        mail_backup_key(&phrase).unwrap().as_bytes()
    );
    assert!(Phrase::parse("not a real phrase at all").is_err());
    let mut words: Vec<&str> = phrase.as_str().split(' ').collect();
    words.swap(0, 1);
    let swapped = words.join(" ");
    if swapped != phrase.as_str() {
        assert!(
            Phrase::parse(&swapped).is_err()
                || RootKey::from_phrase(&Phrase::parse(&swapped).unwrap()).unwrap() != root
        );
    }
}
