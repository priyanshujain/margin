use chacha20poly1305::aead::{Aead, Payload};
use chacha20poly1305::{KeyInit, XChaCha20Poly1305, XNonce};
use rand::RngCore;
use zeroize::Zeroizing;

use crate::{CryptError, Key};

const NONCE_LEN: usize = 24;
const TAG_LEN: usize = 16;
const DATA_KEY_LEN: usize = 32;
const WRAPPED_LEN: usize = NONCE_LEN + DATA_KEY_LEN + TAG_LEN;

const ENVELOPE_MAGIC: &[u8; 4] = b"MCE1";
const MAIL_MAGIC: &[u8; 4] = b"MMB1";

pub fn seal(key: &Key, name: &str, plain: &[u8]) -> Result<Vec<u8>, CryptError> {
    let aad = bound(ENVELOPE_MAGIC, name);
    let mut data_key = Zeroizing::new([0u8; DATA_KEY_LEN]);
    rand::thread_rng().fill_bytes(data_key.as_mut());

    let wrapped = encrypt(key.as_bytes(), data_key.as_ref(), &aad)?;
    let body = encrypt(&data_key, plain, &aad)?;

    let mut out = Vec::with_capacity(ENVELOPE_MAGIC.len() + wrapped.len() + body.len());
    out.extend_from_slice(ENVELOPE_MAGIC);
    out.extend_from_slice(&wrapped);
    out.extend_from_slice(&body);
    Ok(out)
}

pub fn open(key: &Key, name: &str, sealed: &[u8]) -> Result<Vec<u8>, CryptError> {
    if sealed.len() < 4 {
        return Err(CryptError::NotSealed);
    }
    let (magic, rest) = sealed.split_at(4);
    if magic == ENVELOPE_MAGIC {
        if rest.len() < WRAPPED_LEN + NONCE_LEN + TAG_LEN {
            return Err(CryptError::Truncated);
        }
        let aad = bound(ENVELOPE_MAGIC, name);
        let (wrapped, body) = rest.split_at(WRAPPED_LEN);
        let data_key = Zeroizing::new(decrypt(key.as_bytes(), wrapped, &aad)?);
        let data_key: &[u8; DATA_KEY_LEN] = data_key
            .as_slice()
            .try_into()
            .map_err(|_| CryptError::Decrypt)?;
        decrypt(data_key, body, &aad)
    } else if magic == MAIL_MAGIC {
        if rest.len() < NONCE_LEN + TAG_LEN {
            return Err(CryptError::Truncated);
        }
        decrypt(key.as_bytes(), rest, name.as_bytes())
    } else {
        Err(CryptError::NotSealed)
    }
}

fn bound(magic: &[u8; 4], name: &str) -> Vec<u8> {
    let mut aad = Vec::with_capacity(magic.len() + name.len());
    aad.extend_from_slice(magic);
    aad.extend_from_slice(name.as_bytes());
    aad
}

fn encrypt(key: &[u8; 32], plain: &[u8], aad: &[u8]) -> Result<Vec<u8>, CryptError> {
    let mut nonce = [0u8; NONCE_LEN];
    rand::thread_rng().fill_bytes(&mut nonce);
    let body = XChaCha20Poly1305::new(key.into())
        .encrypt((&nonce).into(), Payload { msg: plain, aad })
        .map_err(|_| CryptError::Encrypt)?;
    let mut out = Vec::with_capacity(NONCE_LEN + body.len());
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&body);
    Ok(out)
}

fn decrypt(key: &[u8; 32], sealed: &[u8], aad: &[u8]) -> Result<Vec<u8>, CryptError> {
    let (nonce, body) = sealed.split_at(NONCE_LEN);
    let nonce: &XNonce = nonce.try_into().map_err(|_| CryptError::Truncated)?;
    XChaCha20Poly1305::new(key.into())
        .decrypt(nonce, Payload { msg: body, aad })
        .map_err(|_| CryptError::Decrypt)
}
