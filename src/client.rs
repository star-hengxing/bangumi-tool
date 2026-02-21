use std::time::Duration;

use log::debug;
use tokio::time::sleep;

use crate::error::{AppError, Result};
use crate::models::{PagedCollection, PagedEpisodes, SubjectDetail, User, UserProgress};

const BASE_URL: &str = "https://api.bgm.tv";
const REQUEST_INTERVAL: Duration = Duration::from_secs(5);

pub struct BangumiClient {
    http: reqwest::Client,
    token: String,
}

impl BangumiClient {
    pub fn new(token: String) -> Result<Self> {
        let http = reqwest::Client::builder()
            .user_agent(format!(
                "bangumi-tool/{} (https://github.com/star-hengxing/bangumi-tool)",
                env!("CARGO_PKG_VERSION")
            ))
            .build()?;
        Ok(Self { http, token })
    }

    async fn rate_limit(&self) {
        sleep(REQUEST_INTERVAL).await;
    }

    async fn request(&self, path: &str, query: &[(&str, String)]) -> Result<reqwest::Response> {
        let url = format!("{}{}", BASE_URL, path);
        debug!("GET {} {:?}", url, query);
        let mut builder = self.http.get(&url).bearer_auth(&self.token);
        if !query.is_empty() {
            builder = builder.query(query);
        }
        let resp = builder.send().await?;
        debug!("Response: {} {}", resp.status(), url);
        let status = resp.status();
        if status.is_success() {
            Ok(resp)
        } else {
            let body = resp.text().await.unwrap_or_default();
            debug!("Error body: {}", body);
            Err(AppError::Api {
                status: status.as_u16(),
                message: body,
            })
        }
    }

    pub async fn get_me(&self) -> Result<User> {
        let resp = self.request("/v0/me", &[]).await?;
        Ok(resp.json().await?)
    }

    pub async fn get_collections(
        &self,
        username: &str,
        limit: u64,
        offset: u64,
    ) -> Result<PagedCollection> {
        self.rate_limit().await;
        let path = format!("/v0/users/{}/collections", username);
        let resp = self
            .request(
                &path,
                &[("limit", limit.to_string()), ("offset", offset.to_string())],
            )
            .await?;
        Ok(resp.json().await?)
    }

    pub async fn get_subject(&self, id: u64) -> Result<SubjectDetail> {
        self.rate_limit().await;
        let path = format!("/v0/subjects/{}", id);
        let resp = self.request(&path, &[]).await?;
        Ok(resp.json().await?)
    }

    pub async fn get_episodes(
        &self,
        subject_id: u64,
        limit: u64,
        offset: u64,
    ) -> Result<PagedEpisodes> {
        self.rate_limit().await;
        let resp = self
            .request(
                "/v0/episodes",
                &[
                    ("subject_id", subject_id.to_string()),
                    ("limit", limit.to_string()),
                    ("offset", offset.to_string()),
                ],
            )
            .await?;
        Ok(resp.json().await?)
    }

    pub async fn get_progress(&self, uid: u64, subject_id: u64) -> Result<Option<UserProgress>> {
        self.rate_limit().await;
        let path = format!("/user/{}/progress", uid);
        let resp = match self
            .request(&path, &[("subject_id", subject_id.to_string())])
            .await
        {
            Ok(resp) => resp,
            Err(AppError::Api { status: 404, .. }) => {
                debug!("Progress not found for subject_id={}", subject_id);
                return Ok(None);
            }
            Err(e) => return Err(e),
        };
        let body = resp.text().await?;
        if body == "null" || body.is_empty() {
            debug!("Progress is null for subject_id={}", subject_id);
            return Ok(None);
        }
        let progress: UserProgress = serde_json::from_str(&body)?;
        Ok(Some(progress))
    }
}
