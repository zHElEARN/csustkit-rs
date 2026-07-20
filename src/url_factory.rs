use crate::connection::ConnectionMode;

const WEBVPN_BASE: &str = "https://vpn.csust.edu.cn";
const WEBVPN_PREFIX: &str = "webvpn";

#[derive(Clone, Copy, Debug)]
pub(crate) enum ServiceDomain {
    AuthServer,
    CampusCard,
    Education,
    Ehall,
    Mooc,
}

impl ServiceDomain {
    fn scheme(self) -> &'static str {
        match self {
            Self::AuthServer | Self::CampusCard | Self::Ehall => "https",
            Self::Education | Self::Mooc => "http",
        }
    }

    fn direct_host(self) -> &'static str {
        match self {
            Self::AuthServer => "authserver.csust.edu.cn",
            Self::CampusCard => "hxyxh5.csust.edu.cn",
            Self::Education => "xk.csust.edu.cn",
            Self::Ehall => "ehall.csust.edu.cn",
            Self::Mooc => "pt.csust.edu.cn",
        }
    }

    fn vpn_hex(self) -> &'static str {
        match self {
            Self::AuthServer => "b9fbab94ec37584ef499d74673ec2c940949105c7b30eca147702d9482299f99",
            Self::CampusCard => "6a312b2d860191c92db8c011e7e418eac2691c647e6e2b00de67552d70884967",
            Self::Education => "505c0e70383db2ebb7035169513d1ffa",
            Self::Ehall => "1e2b5c384f0dc42e4d0db781d590f8e2f8f129ae812718586ddba3948db7b103",
            Self::Mooc => "ca1e69080fcc45ac45bed760950fd677",
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
        assert_eq!(
            make_url(ConnectionMode::Direct, ServiceDomain::Education, "/sso.jsp"),
            "http://xk.csust.edu.cn/sso.jsp",
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
        assert_eq!(
            make_url(
                ConnectionMode::WebVpn,
                ServiceDomain::Mooc,
                "/meol/index.do"
            ),
            "https://vpn.csust.edu.cn/http/webvpnca1e69080fcc45ac45bed760950fd677/meol/index.do",
        );
        assert_eq!(
            make_url(ConnectionMode::WebVpn, ServiceDomain::Education, "/sso.jsp"),
            "https://vpn.csust.edu.cn/http/webvpn505c0e70383db2ebb7035169513d1ffa/sso.jsp",
        );
    }
}
