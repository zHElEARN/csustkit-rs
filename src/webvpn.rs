use aes::cipher::{BlockModeDecrypt, BlockModeEncrypt, KeyIvInit, block_padding::Pkcs7};
use url::Url;

type Aes128CbcEnc = cbc::Encryptor<aes::Aes128>;
type Aes128CbcDec = cbc::Decryptor<aes::Aes128>;

const WEBVPN_HOST: &str = "vpn.csust.edu.cn";
const WEBVPN_KEY: &[u8; 16] = b"CASB2021EnLink!!";
const WEBVPN_IV: &[u8; 16] = b"CASB2021EnLink!!";
const WEBVPN_PREFIX: &str = "webvpn";

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error, uniffi::Error)]
pub enum WebVpnError {
    #[error("URL encryption failed")]
    UrlEncryptionFailed,
    #[error("URL decryption failed")]
    UrlDecryptionFailed,
    #[error("Host encryption failed")]
    HostEncryptionFailed,
    #[error("Host decryption failed")]
    HostDecryptionFailed,
}

#[uniffi::export]
pub fn webvpn_encrypt_url(original_url: String) -> Result<String, WebVpnError> {
    let url = Url::parse(&original_url).map_err(|_| WebVpnError::UrlEncryptionFailed)?;
    let scheme = url.scheme();
    let host = url.host_str().ok_or(WebVpnError::UrlEncryptionFailed)?;

    let mut original_host = host.to_owned();
    if let Some(port) = url.port() {
        original_host.push(':');
        original_host.push_str(&port.to_string());
    }

    let encrypted_host = encrypt_host(&original_host)?;
    let final_path = match url.path() {
        "" => "/",
        path if path.starts_with('/') => path,
        _ => return Err(WebVpnError::UrlEncryptionFailed),
    };

    let mut encrypted_url =
        format!("https://{WEBVPN_HOST}/{scheme}/{WEBVPN_PREFIX}{encrypted_host}{final_path}");
    append_query_and_fragment(&mut encrypted_url, url.query(), url.fragment());

    Url::parse(&encrypted_url).map_err(|_| WebVpnError::UrlEncryptionFailed)?;
    Ok(encrypted_url)
}

#[uniffi::export]
pub fn webvpn_decrypt_url(vpn_url: String) -> Result<String, WebVpnError> {
    let url = Url::parse(&vpn_url).map_err(|_| WebVpnError::UrlDecryptionFailed)?;
    let path = url.path();
    let path_components = path.split('/').collect::<Vec<_>>();

    if path_components.len() < 3 {
        return Err(WebVpnError::UrlDecryptionFailed);
    }

    let scheme = path_components[1];
    let encrypted_host_component = path_components[2];

    let encrypted_host = encrypted_host_component
        .strip_prefix(WEBVPN_PREFIX)
        .ok_or(WebVpnError::UrlDecryptionFailed)?;
    let decrypted_host = decrypt_host(encrypted_host)?;

    let mut host_parts = decrypted_host.split(':');
    let host = host_parts
        .next()
        .filter(|host| !host.is_empty())
        .ok_or(WebVpnError::UrlDecryptionFailed)?;
    let port = host_parts.next().and_then(|port| port.parse::<u16>().ok());

    let prefix_to_drop = format!("/{scheme}/{encrypted_host_component}");
    if !path.starts_with(&prefix_to_drop) {
        return Err(WebVpnError::UrlDecryptionFailed);
    }

    let dropped_path = &path[prefix_to_drop.len()..];
    let original_path = if dropped_path == "/" {
        String::new()
    } else if !dropped_path.is_empty() && !dropped_path.starts_with('/') {
        format!("/{dropped_path}")
    } else {
        dropped_path.to_owned()
    };

    let mut original_url = format!("{scheme}://{host}");
    if let Some(port) = port {
        original_url.push(':');
        original_url.push_str(&port.to_string());
    }
    original_url.push_str(&original_path);
    append_query_and_fragment(&mut original_url, url.query(), url.fragment());

    Url::parse(&original_url).map_err(|_| WebVpnError::UrlDecryptionFailed)?;
    Ok(original_url)
}

fn encrypt_host(text: &str) -> Result<String, WebVpnError> {
    let encrypted = Aes128CbcEnc::new_from_slices(WEBVPN_KEY, WEBVPN_IV)
        .map_err(|_| WebVpnError::HostEncryptionFailed)?
        .encrypt_padded_vec::<Pkcs7>(text.as_bytes());
    Ok(hex::encode(encrypted))
}

fn decrypt_host(hex_text: &str) -> Result<String, WebVpnError> {
    let encrypted = hex::decode(hex_text).map_err(|_| WebVpnError::HostDecryptionFailed)?;
    let decrypted = Aes128CbcDec::new_from_slices(WEBVPN_KEY, WEBVPN_IV)
        .map_err(|_| WebVpnError::HostDecryptionFailed)?
        .decrypt_padded_vec::<Pkcs7>(&encrypted)
        .map_err(|_| WebVpnError::HostDecryptionFailed)?;

    String::from_utf8(decrypted).map_err(|_| WebVpnError::HostDecryptionFailed)
}

fn append_query_and_fragment(url: &mut String, query: Option<&str>, fragment: Option<&str>) {
    if let Some(query) = query {
        url.push('?');
        url.push_str(query);
    }
    if let Some(fragment) = fragment {
        url.push('#');
        url.push_str(fragment);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decrypts_known_webvpn_urls() {
        let cases = [
            (
                "https://vpn.csust.edu.cn/http/webvpnca1e69080fcc45ac45bed760950fd677/",
                "http://pt.csust.edu.cn",
            ),
            (
                "https://vpn.csust.edu.cn/http/webvpn505c0e70383db2ebb7035169513d1ffa/",
                "http://xk.csust.edu.cn",
            ),
            (
                "https://vpn.csust.edu.cn/http/webvpn0290db6ae56290b8883befe06cf1faf082ff567793ebc8ea223c577d2c216af3/index.html",
                "http://192.168.1.1:8080/index.html",
            ),
        ];

        for (vpn_url, original_url) in cases {
            assert_eq!(
                webvpn_decrypt_url(vpn_url.to_owned()).unwrap(),
                original_url
            );
        }
    }

    #[test]
    fn encrypts_known_original_urls() {
        let cases = [
            (
                "http://xk.csust.edu.cn",
                "https://vpn.csust.edu.cn/http/webvpn505c0e70383db2ebb7035169513d1ffa/",
            ),
            (
                "http://pt.csust.edu.cn",
                "https://vpn.csust.edu.cn/http/webvpnca1e69080fcc45ac45bed760950fd677/",
            ),
            (
                "http://192.168.1.1:8080/index.html",
                "https://vpn.csust.edu.cn/http/webvpn0290db6ae56290b8883befe06cf1faf082ff567793ebc8ea223c577d2c216af3/index.html",
            ),
        ];

        for (original_url, vpn_url) in cases {
            assert_eq!(
                webvpn_encrypt_url(original_url.to_owned()).unwrap(),
                vpn_url
            );
        }
    }

    #[test]
    fn round_trips_common_urls() {
        let urls = [
            "http://www.baidu.com",
            "https://jwc.csust.edu.cn",
            "http://192.168.1.1:8080/index.html",
            "https://lofter.com/front/login?id=123",
        ];

        for url in urls {
            let encrypted = webvpn_encrypt_url(url.to_owned()).unwrap();
            assert_eq!(webvpn_decrypt_url(encrypted).unwrap(), url);
        }
    }

    #[test]
    fn encrypt_reports_url_errors() {
        assert_eq!(
            webvpn_encrypt_url("not a url".to_owned()).unwrap_err(),
            WebVpnError::UrlEncryptionFailed
        );
        assert_eq!(
            webvpn_encrypt_url("mailto:hello@example.com".to_owned()).unwrap_err(),
            WebVpnError::UrlEncryptionFailed
        );
    }

    #[test]
    fn decrypt_reports_url_errors() {
        assert_eq!(
            webvpn_decrypt_url("https://vpn.csust.edu.cn/http".to_owned()).unwrap_err(),
            WebVpnError::UrlDecryptionFailed
        );
        assert_eq!(
            webvpn_decrypt_url("https://vpn.csust.edu.cn/http/notwebvpnabc/".to_owned())
                .unwrap_err(),
            WebVpnError::UrlDecryptionFailed
        );
    }

    #[test]
    fn decrypt_reports_host_errors() {
        assert_eq!(
            webvpn_decrypt_url("https://vpn.csust.edu.cn/http/webvpnnot-hex/".to_owned())
                .unwrap_err(),
            WebVpnError::HostDecryptionFailed
        );
        assert_eq!(
            webvpn_decrypt_url("https://vpn.csust.edu.cn/http/webvpn00/".to_owned()).unwrap_err(),
            WebVpnError::HostDecryptionFailed
        );
    }
}
