use aes::cipher::{BlockModeEncrypt, KeyIvInit, block_padding::Pkcs7};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};

type Aes128CbcEnc = cbc::Encryptor<aes::Aes128>;

const RANDOM_CHARS: &[u8] = b"ABCDEFGHJKMNPQRSTWXYZabcdefhijkmnprstwxyz2345678";

pub(super) fn encrypt_password(password: &str, salt: &str) -> Result<String, ()> {
    if salt.is_empty() {
        return Ok(password.to_owned());
    }

    let prefix = random_string(64)?;
    let iv = random_string(16)?;
    encrypt_password_with_parts(password, salt, &prefix, &iv)
}

fn encrypt_password_with_parts(
    password: &str,
    salt: &str,
    prefix: &str,
    iv: &str,
) -> Result<String, ()> {
    let mut key = [0u8; 16];
    let salt_bytes = salt.as_bytes();
    if salt_bytes.len() < key.len() {
        return Err(());
    }
    key.copy_from_slice(&salt_bytes[..16]);

    let mut plain_text = prefix.to_owned();
    plain_text.push_str(password);

    let encrypted = Aes128CbcEnc::new_from_slices(&key, iv.as_bytes())
        .map_err(|_| ())?
        .encrypt_padded_vec::<Pkcs7>(plain_text.as_bytes());
    Ok(BASE64_STANDARD.encode(encrypted))
}

fn random_string(length: usize) -> Result<String, ()> {
    let mut bytes = vec![0u8; length];
    getrandom::fill(&mut bytes).map_err(|_| ())?;

    Ok(bytes
        .into_iter()
        .map(|byte| RANDOM_CHARS[usize::from(byte) % RANDOM_CHARS.len()] as char)
        .collect())
}
