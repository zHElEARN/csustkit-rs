use std::sync::RwLock;

use serde::de::DeserializeOwned;

use crate::{
    connection::ConnectionMode,
    session::CsustSession,
    url_factory::{ServiceDomain, make_url},
};

use super::{
    protocol::{
        BaseQueryResponse, BaseResponse, QueryItem, RoomPowerInfo, TokenResponse,
        profile_from_response, remaining_electricity,
    },
    types::{Building, Campus, CampusCardError, Profile, Room},
};

const TOKEN_AUTHORIZATION: &str =
    "Basic bW9iaWxlX3NlcnZpY2VfcGxhdGZvcm06bW9iaWxlX3NlcnZpY2VfcGxhdGZvcm1fc2VjcmV0";
const QUERY_AUTHORIZATION: &str = "Y2hhcmdlOmNoYXJnZV9zZWNyZXQ=";

pub struct CampusCardHelper {
    mode: ConnectionMode,
    session: CsustSession,
    token: RwLock<Option<String>>,
}

impl CampusCardHelper {
    pub fn new(mode: ConnectionMode) -> Result<Self, CampusCardError> {
        let session = CsustSession::new()
            .map_err(|error| CampusCardError::ClientBuildFailed(error.to_string()))?;
        Ok(Self::with_session(mode, session))
    }

    pub fn with_session(mode: ConnectionMode, session: CsustSession) -> Self {
        Self {
            mode,
            session,
            token: RwLock::new(None),
        }
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
