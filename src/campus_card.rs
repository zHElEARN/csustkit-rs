use std::sync::{Arc, RwLock};

use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::{
    connection::ConnectionMode,
    session::CsustSession,
    url_factory::{ServiceDomain, make_url},
};

const TOKEN_AUTHORIZATION: &str =
    "Basic bW9iaWxlX3NlcnZpY2VfcGxhdGZvcm06bW9iaWxlX3NlcnZpY2VfcGxhdGZvcm1fc2VjcmV0";
const QUERY_AUTHORIZATION: &str = "Y2hhcmdlOmNoYXJnZV9zZWNyZXQ=";

#[derive(Debug, thiserror::Error)]
pub enum CampusCardError {
    #[error("校园卡客户端创建失败: {0}")]
    ClientBuildFailed(String),
    #[error("个人信息获取失败: {0}")]
    ProfileRetrievalFailed(String),
    #[error("楼栋信息获取失败: {0}")]
    BuildingsRetrievalFailed(String),
    #[error("宿舍列表获取失败: {0}")]
    RoomsRetrievalFailed(String),
    #[error("宿舍电量获取失败: {0}")]
    ElectricityRetrievalFailed(String),
    #[error("校园卡系统未登录")]
    NotLoggedIn,
    #[error(transparent)]
    Network(#[from] reqwest::Error),
}

/// 校区。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Campus {
    Yuntang,
    Jinpenling,
}

impl Campus {
    pub const ALL: [Self; 2] = [Self::Yuntang, Self::Jinpenling];

    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Yuntang => "云塘校区",
            Self::Jinpenling => "金盆岭校区",
        }
    }

    const fn fee_item_id(self) -> &'static str {
        match self {
            Self::Yuntang => "448",
            Self::Jinpenling => "468",
        }
    }

    const fn campus_id(self) -> &'static str {
        match self {
            Self::Yuntang => "1",
            Self::Jinpenling => "22",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Building {
    pub name: String,
    pub id: String,
    pub campus: Campus,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Room {
    pub name: String,
    pub id: String,
    pub building: Building,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: i64,
    pub account: String,
    pub name: String,
    pub avatar: String,
    pub email: Option<String>,
    pub mobile: Option<String>,
    pub user_type: String,
    pub sno: String,
    pub card_id: String,
    pub card_account: String,
    pub sex: i64,
    pub country_code: String,
    pub nation_code: String,
    pub politics_status_code: String,
    pub identity_code: String,
    pub campus_code: String,
    pub department_code: String,
    pub department_name: String,
    pub profession_code: String,
    pub profession_name: String,
    pub class_code: String,
    pub class_name: String,
    pub flag: String,
    pub user_id: i64,
    pub id_number: String,
    pub identity_name: String,
    pub identity_exp_date: String,
    pub remark: String,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
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
struct BaseResponse<T> {
    code: i64,
    success: Option<bool>,
    data: Option<T>,
    msg: Option<String>,
    message: Option<String>,
}

#[derive(Deserialize)]
struct MapContainer<T> {
    data: T,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct BaseQueryResponse<T> {
    msg: Option<String>,
    message: Option<String>,
    code: i64,
    map: Option<MapContainer<T>>,
}

#[derive(Deserialize)]
struct QueryItem {
    name: String,
    value: String,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct RoomPowerInfo {
    room_id: String,
    #[serde(rename = "allAmp")]
    all_amp: String,
    xiaoqu_id: String,
    #[serde(rename = "usedAmp")]
    used_amp: String,
    loudong_id: String,
}

pub struct CampusCardHelper {
    mode: ConnectionMode,
    session: Arc<CsustSession>,
    token: RwLock<Option<String>>,
}

impl CampusCardHelper {
    pub fn new(mode: ConnectionMode) -> Result<Arc<Self>, CampusCardError> {
        let session = CsustSession::new()
            .map_err(|error| CampusCardError::ClientBuildFailed(error.to_string()))?;
        Ok(Self::with_session(mode, session))
    }

    pub fn with_session(mode: ConnectionMode, session: Arc<CsustSession>) -> Arc<Self> {
        Arc::new(Self {
            mode,
            session,
            token: RwLock::new(None),
        })
    }

    /// 返回当前 token 的快照。
    pub fn token(&self) -> Option<String> {
        match self.token.read() {
            Ok(token) => token.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        }
    }

    /// 替换或清空当前 token。
    pub fn set_token(&self, token: Option<String>) {
        match self.token.write() {
            Ok(mut current) => *current = token,
            Err(poisoned) => *poisoned.into_inner() = token,
        }
    }

    pub async fn sync_token(&self, ticket: String) -> Result<(), CampusCardError> {
        let parameters = [
            ("username", ticket.as_str()),
            ("password", ticket.as_str()),
            ("grant_type", "password"),
            ("scope", "all"),
            ("loginFrom", "h5"),
            ("logintype", "sso"),
            ("device_token", "h5"),
            ("synAccessSource", "h5"),
        ];
        let response = self
            .session
            .client
            .post(make_url(
                self.mode,
                ServiceDomain::CampusCard,
                "/berserker-auth/oauth/token",
            ))
            .header("Authorization", TOKEN_AUTHORIZATION)
            .form(&parameters)
            .send()
            .await?
            .json::<TokenResponse>()
            .await?;

        self.set_token(Some(response.access_token));
        Ok(())
    }

    pub async fn get_profile(&self) -> Result<Profile, CampusCardError> {
        let token = self.required_token()?;
        let response = self
            .session
            .client
            .get(make_url(
                self.mode,
                ServiceDomain::CampusCard,
                "/berserker-base/user?synAccessSource=h5",
            ))
            .header("synjones-auth", format!("bearer {token}"))
            .send()
            .await?
            .json::<BaseResponse<Profile>>()
            .await?;

        profile_from_response(response)
    }

    pub async fn get_buildings(&self, campus: Campus) -> Result<Vec<Building>, CampusCardError> {
        let token = self.required_token()?;
        let parameters = [
            ("feeitemid", campus.fee_item_id()),
            ("type", "select"),
            ("level", "1"),
            ("xiaoqu_id", campus.campus_id()),
        ];
        let response = self
            .get_third_data::<Vec<QueryItem>>(&token, &parameters)
            .await?;

        if response.code == 401 {
            return Err(CampusCardError::NotLoggedIn);
        }
        let items = response.map.ok_or_else(|| {
            CampusCardError::BuildingsRetrievalFailed(format!(
                "获取楼栋列表失败: {}",
                response.message.as_deref().unwrap_or("nil")
            ))
        })?;

        Ok(items
            .data
            .into_iter()
            .map(|item| Building {
                name: item.name,
                id: item.value,
                campus,
            })
            .collect())
    }

    pub async fn get_rooms(&self, building: &Building) -> Result<Vec<Room>, CampusCardError> {
        let token = self.required_token()?;
        let parameters = [
            ("feeitemid", building.campus.fee_item_id()),
            ("type", "select"),
            ("level", "2"),
            ("xiaoqu_id", building.campus.campus_id()),
            ("loudong_id", building.id.as_str()),
        ];
        let response = self
            .get_third_data::<Vec<QueryItem>>(&token, &parameters)
            .await?;

        if response.code == 401 {
            return Err(CampusCardError::NotLoggedIn);
        }
        let items = response.map.ok_or_else(|| {
            CampusCardError::RoomsRetrievalFailed(format!(
                "获取宿舍列表失败: {}",
                response.message.as_deref().unwrap_or("nil")
            ))
        })?;

        Ok(items
            .data
            .into_iter()
            .map(|item| Room {
                name: item.name,
                id: item.value,
                building: building.clone(),
            })
            .collect())
    }

    pub async fn get_electricity(&self, room: &Room) -> Result<f64, CampusCardError> {
        let token = self.required_token()?;
        let parameters = [
            ("feeitemid", room.building.campus.fee_item_id()),
            ("type", "IEC"),
            ("level", "3"),
            ("xiaoqu_id", room.building.campus.campus_id()),
            ("loudong_id", room.building.id.as_str()),
            ("room_id", room.id.as_str()),
        ];
        let response = self
            .get_third_data::<RoomPowerInfo>(&token, &parameters)
            .await?;

        if response.code == 401 {
            return Err(CampusCardError::NotLoggedIn);
        }
        let power = response.map.ok_or_else(|| {
            CampusCardError::ElectricityRetrievalFailed(format!(
                "获取宿舍电量失败: {}",
                response.message.as_deref().unwrap_or("nil")
            ))
        })?;

        remaining_electricity(&power.data)
    }

    pub async fn is_logged_in(&self) -> bool {
        self.get_profile().await.is_ok()
    }

    pub async fn logout(&self) -> Result<(), CampusCardError> {
        let token = self.token().unwrap_or_default();
        self.session
            .client
            .get(make_url(
                self.mode,
                ServiceDomain::CampusCard,
                &format!(
                    "/berserker-base/redirect?type=logout&synjones-auth={token}&loginFrom=h5&synAccessSource=h5"
                ),
            ))
            .send()
            .await?
            .bytes()
            .await?;

        self.set_token(None);
        Ok(())
    }

    fn required_token(&self) -> Result<String, CampusCardError> {
        self.token().ok_or(CampusCardError::NotLoggedIn)
    }

    async fn get_third_data<T: DeserializeOwned>(
        &self,
        token: &str,
        parameters: &[(&str, &str)],
    ) -> Result<BaseQueryResponse<T>, CampusCardError> {
        let response = self
            .session
            .client
            .post(make_url(
                self.mode,
                ServiceDomain::CampusCard,
                "/charge/feeitem/getThirdData",
            ))
            .header("Authorization", QUERY_AUTHORIZATION)
            .header("synjones-auth", format!("bearer {token}"))
            .form(parameters)
            .send()
            .await?
            .json::<BaseQueryResponse<T>>()
            .await?;

        Ok(response)
    }
}

fn profile_from_response(response: BaseResponse<Profile>) -> Result<Profile, CampusCardError> {
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

fn remaining_electricity(power: &RoomPowerInfo) -> Result<f64, CampusCardError> {
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
