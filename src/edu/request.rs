use std::time::Duration;

use reqwest::RequestBuilder;
use tokio::time::sleep;

use crate::request::{LoginCheck, RequestResponse, perform_request};

use super::EducationRequestError;

const MAX_RETRIES: u8 = 5;
const RETRY_DELAY: Duration = Duration::from_secs(1);

pub(crate) async fn send_with_retry<E>(
    build_request: impl Fn() -> RequestBuilder,
    login_check: impl Fn() -> LoginCheck<E>,
) -> Result<RequestResponse, E>
where
    E: From<reqwest::Error> + From<EducationRequestError>,
{
    let mut retries = 0_u8;
    loop {
        let response = perform_request(build_request(), Some(login_check())).await?;

        if !response.body.is_empty() {
            return Ok(response);
        }

        if retries >= MAX_RETRIES {
            return Err(EducationRequestError::EmptyResponse {
                url: response.final_url,
                attempts: retries + 1,
            }
            .into());
        }

        retries += 1;
        sleep(RETRY_DELAY).await;
    }
}
