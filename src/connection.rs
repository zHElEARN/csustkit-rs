#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum ConnectionMode {
    Direct,
    WebVpn,
}
