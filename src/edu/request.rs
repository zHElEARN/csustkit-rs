use std::time::Duration;

use reqwest::RequestBuilder;
use tokio::time::sleep;

use super::EducationRequestError;

const MAX_RETRIES: u8 = 5;
const RETRY_DELAY: Duration = Duration::from_secs(1);

pub(crate) struct EducationResponse {
    pub(crate) body: Vec<u8>,
}

pub(crate) async fn send_with_retry<F>(
    build_request: F,
) -> Result<EducationResponse, EducationRequestError>
where
    F: Fn() -> RequestBuilder,
{
    let mut retries = 0_u8;
    loop {
        let response = build_request().send().await?;
        let url = response.url().clone();
        let body = response.bytes().await?.to_vec();

        if !body.is_empty() {
            return Ok(EducationResponse { body });
        }

        if retries >= MAX_RETRIES {
            return Err(EducationRequestError::EmptyResponse {
                url,
                attempts: retries + 1,
            });
        }

        retries += 1;
        sleep(RETRY_DELAY).await;
    }
}
