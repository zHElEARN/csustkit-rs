use crate::connection::ConnectionMode;

const WEBVPN_BASE: &str = "https://vpn.csust.edu.cn";
const WEBVPN_PREFIX: &str = "webvpn";

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
pub enum ServiceDomain {
    AuthServer,
    CampusCard,
    Chaoxing,
    ChaoxingApi,
    Education,
    Ehall,
    Eval,
    Mooc,
    PhysicsExperiment,
}

impl ServiceDomain {
    fn scheme(self) -> &'static str {
        match self {
            Self::AuthServer
            | Self::CampusCard
            | Self::Chaoxing
            | Self::ChaoxingApi
            | Self::Ehall
            | Self::Eval => "https",
            Self::Education | Self::Mooc | Self::PhysicsExperiment => "http",
        }
    }

    fn direct_host(self) -> &'static str {
        match self {
            Self::AuthServer => "authserver.csust.edu.cn",
            Self::CampusCard => "hxyxh5.csust.edu.cn",
            Self::Chaoxing => "i.mooc.chaoxing.com",
            Self::ChaoxingApi => "mooc1-api.chaoxing.com",
            Self::Education => "xk.csust.edu.cn",
            Self::Ehall => "ehall.csust.edu.cn",
            Self::Eval => "zbxt.csust.edu.cn",
            Self::Mooc => "pt.csust.edu.cn",
            Self::PhysicsExperiment => "10.255.65.52",
        }
    }

    fn vpn_hex(self) -> &'static str {
        match self {
            Self::AuthServer => "b9fbab94ec37584ef499d74673ec2c940949105c7b30eca147702d9482299f99",
            Self::CampusCard => "6a312b2d860191c92db8c011e7e418eac2691c647e6e2b00de67552d70884967",
            Self::Chaoxing => "05cc0efef664f2f86e16be7e169783e378f442c81ff65a9b669a92968c03bb85",
            Self::ChaoxingApi => "d529fd82bb238cec8e955f0dcc78df9c9c3b68d58d3df59d4c7d1ec8318c66d6",
            Self::Education => "505c0e70383db2ebb7035169513d1ffa",
            Self::Ehall => "1e2b5c384f0dc42e4d0db781d590f8e2f8f129ae812718586ddba3948db7b103",
            Self::Eval => "e3e5295206d19ea6a84a70370bb4a4db1c0176decbcfae350b6673a608d0d751",
            Self::Mooc => "ca1e69080fcc45ac45bed760950fd677",
            Self::PhysicsExperiment => "ee536efb7808aac9b0bc36403333c380",
        }
    }
}

#[cfg_attr(feature = "uniffi", uniffi::export)]
pub fn make_url(mode: ConnectionMode, domain: ServiceDomain, path: &str) -> String {
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
    use crate::{ConnectionMode, ServiceDomain, make_url};

    #[test]
    fn builds_eval_direct_url_without_leading_slash() {
        assert_eq!(
            make_url(
                ConnectionMode::Direct,
                ServiceDomain::Eval,
                "api/manage/cas/toUrl?type=pc"
            ),
            "https://zbxt.csust.edu.cn/api/manage/cas/toUrl?type=pc"
        );
    }

    #[test]
    fn builds_eval_webvpn_url_with_query() {
        assert_eq!(
            make_url(
                ConnectionMode::WebVpn,
                ServiceDomain::Eval,
                "/api/manage/cas/toUrl?type=pc"
            ),
            concat!(
                "https://vpn.csust.edu.cn/https/webvpn",
                "e3e5295206d19ea6a84a70370bb4a4db1c0176decbcfae350b6673a608d0d751",
                "/api/manage/cas/toUrl?type=pc"
            )
        );
    }
}
