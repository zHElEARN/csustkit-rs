use serde::Deserialize;

use super::types::{CampusCardError, Profile};

#[allow(dead_code)]
#[derive(Deserialize)]
pub(super) struct TokenResponse {
    pub(super) access_token: String,
    token_type: String,
    refresh_token: String,
    expires_in: i64,
    scope: String,
    tenant_id: String,
    is_first_login: bool,
    flag: String,
    sno: String,
    logintype: String,
    name: String,
    mobile: String,
    id: i64,
    #[serde(rename = "loginFrom")]
    login_from: String,
    uuid: String,
    client_id: String,
    is_password_expired: bool,
    jti: String,
}

#[allow(dead_code)]
#[derive(Deserialize)]
pub(super) struct BaseResponse<T> {
    pub(super) code: i64,
    success: Option<bool>,
    pub(super) data: Option<T>,
    msg: Option<String>,
    pub(super) message: Option<String>,
}

#[derive(Deserialize)]
pub(super) struct MapContainer<T> {
    pub(super) data: T,
}

#[allow(dead_code)]
#[derive(Deserialize)]
pub(super) struct BaseQueryResponse<T> {
    msg: Option<String>,
    pub(super) message: Option<String>,
    pub(super) code: i64,
    pub(super) map: Option<MapContainer<T>>,
}

#[derive(Deserialize)]
pub(super) struct QueryItem {
    pub(super) name: String,
    pub(super) value: String,
}

#[allow(dead_code)]
#[derive(Deserialize)]
pub(super) struct RoomPowerInfo {
    room_id: String,
    #[serde(rename = "allAmp")]
    pub(super) all_amp: String,
    xiaoqu_id: String,
    #[serde(rename = "usedAmp")]
    pub(super) used_amp: String,
    loudong_id: String,
}

pub(super) fn profile_from_response(
    response: BaseResponse<Profile>,
) -> Result<Profile, CampusCardError> {
    if response.code == 401 {
        return Err(CampusCardError::NotLoggedIn);
    }

    response.data.ok_or_else(|| {
        CampusCardError::ProfileRetrievalFailed(format!(
            "获取个人信息失败: {}",
            response.message.as_deref().unwrap_or("nil")
        ))
    })
}

pub(super) fn remaining_electricity(power: &RoomPowerInfo) -> Result<f64, CampusCardError> {
    let all = power
        .all_amp
        .parse::<f64>()
        .map_err(|_| CampusCardError::ElectricityRetrievalFailed("无法解析电量".to_owned()))?;
    let used = power
        .used_amp
        .parse::<f64>()
        .map_err(|_| CampusCardError::ElectricityRetrievalFailed("无法解析电量".to_owned()))?;
    Ok(all - used)
}
