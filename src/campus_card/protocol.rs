use serde::Deserialize;

use super::types::{CampusCardError, Profile};

#[derive(Deserialize)]
pub(super) struct TokenResponse {
    pub(super) access_token: String,
}

#[derive(Deserialize)]
pub(super) struct BaseResponse<T> {
    pub(super) data: Option<T>,
    pub(super) message: Option<String>,
}

#[derive(Deserialize)]
pub(super) struct MapContainer<T> {
    pub(super) data: T,
}

#[derive(Deserialize)]
pub(super) struct BaseQueryResponse<T> {
    pub(super) message: Option<String>,
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

pub(super) fn is_login_response(body: &str) -> bool {
    #[derive(Deserialize)]
    struct ResponseCode {
        code: i64,
    }

    serde_json::from_str::<ResponseCode>(body)
        .is_ok_and(|response| matches!(response.code, 400 | 401))
}

pub(super) fn profile_from_response(
    response: BaseResponse<Profile>,
) -> Result<Profile, CampusCardError> {
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
