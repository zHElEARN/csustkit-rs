use crate::connection::ConnectionMode;

const WEBVPN_BASE: &str = "https://vpn.csust.edu.cn";
const WEBVPN_PREFIX: &str = "webvpn";

#[derive(Clone, Copy, Debug)]
pub(crate) enum ServiceDomain {
    AuthServer,
    Ehall,
}

impl ServiceDomain {
    fn scheme(self) -> &'static str {
        match self {
            Self::AuthServer | Self::Ehall => "https",
        }
    }

    fn direct_host(self) -> &'static str {
        match self {
            Self::AuthServer => "authserver.csust.edu.cn",
            Self::Ehall => "ehall.csust.edu.cn",
        }
    }

    fn vpn_hex(self) -> &'static str {
        match self {
            Self::AuthServer => "b9fbab94ec37584ef499d74673ec2c940949105c7b30eca147702d9482299f99",
            Self::Ehall => "1e2b5c384f0dc42e4d0db781d590f8e2f8f129ae812718586ddba3948db7b103",
        }
    }
}

pub(crate) fn make_url(mode: ConnectionMode, domain: ServiceDomain, path: &str) -> String {
    let safe_path = if path.starts_with('/') {
        path.to_owned()
    } else {
        format!("/{path}")
    };

    match mode {
        ConnectionMode::Direct => {
            format!(
                "{}://{}{}",
                domain.scheme(),
                domain.direct_host(),
                safe_path
            )
        }
        ConnectionMode::WebVpn => {
            format!(
                "{WEBVPN_BASE}/{}/{WEBVPN_PREFIX}{}{}",
                domain.scheme(),
                domain.vpn_hex(),
                safe_path
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_direct_urls() {
        assert_eq!(
            make_url(
                ConnectionMode::Direct,
                ServiceDomain::AuthServer,
                "/authserver/login?service=https%3A%2F%2Fehall.csust.edu.cn%2Flogin",
            ),
            "https://authserver.csust.edu.cn/authserver/login?service=https%3A%2F%2Fehall.csust.edu.cn%2Flogin",
        );
        assert_eq!(
            make_url(ConnectionMode::Direct, ServiceDomain::Ehall, "index.html"),
            "https://ehall.csust.edu.cn/index.html",
        );
    }

    #[test]
    fn builds_webvpn_urls() {
        assert_eq!(
            make_url(
                ConnectionMode::WebVpn,
                ServiceDomain::AuthServer,
                "/authserver/login?service=https%3A%2F%2Fehall.csust.edu.cn%2Flogin",
            ),
            "https://vpn.csust.edu.cn/https/webvpnb9fbab94ec37584ef499d74673ec2c940949105c7b30eca147702d9482299f99/authserver/login?service=https%3A%2F%2Fehall.csust.edu.cn%2Flogin",
        );
        assert_eq!(
            make_url(ConnectionMode::WebVpn, ServiceDomain::Ehall, "/index.html"),
            "https://vpn.csust.edu.cn/https/webvpn1e2b5c384f0dc42e4d0db781d590f8e2f8f129ae812718586ddba3948db7b103/index.html",
        );
    }
}
