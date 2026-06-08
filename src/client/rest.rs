//! Low-level REST client for Webex API requests.

use crate::error::Error;
use crate::types::{EmptyReply, Gettable, ListResult};
use log::{error, trace};
use parse_link_header::parse_with_rel;
use reqwest::header::LINK;
use reqwest::StatusCode;
use serde::{de::DeserializeOwned, Serialize};
use serde_json;
use std::collections::HashMap;

const MAX_NEXT_PAGE_REQUESTS: usize = 100;

/// Authorization type for REST requests.
#[derive(Clone, Copy)]
pub enum AuthorizationType<'a> {
    /// No authorization
    None,
    /// Bearer token authorization
    Bearer(&'a str),
    /// Basic authentication
    Basic {
        /// Username
        username: &'a str,
        /// Password
        password: &'a str,
    },
}

/// Body type for REST requests.
enum Body<T: Serialize> {
    Json(T),
    UrlEncoded(T),
}

const BODY_NONE: Option<Body<()>> = None;

/// Implements low level REST requests to be used internally by the library.
#[derive(Clone)]
pub struct RestClient {
    /// Host prefix mapping for different API endpoints
    pub host_prefix: HashMap<String, String>,
    /// Underlying HTTP client
    pub web_client: reqwest::Client,
}

impl RestClient {
    /// Creates a new `RestClient`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            host_prefix: HashMap::new(),
            web_client: reqwest::Client::new(),
        }
    }

    /// Creates a `RestClient` with existing `host_prefix` and `web_client`.
    #[must_use]
    pub const fn new_with(
        host_prefix: HashMap<String, String>,
        web_client: reqwest::Client,
    ) -> Self {
        Self {
            host_prefix,
            web_client,
        }
    }

    /// Performs a GET request and returns the resource as JSON.
    ///
    /// # Arguments
    ///
    /// * `token` - Authorization token
    /// * `full_url` - Full URL to GET (e.g. `<https://api.ciscospark.com/v1/messages/abc123`>)
    pub async fn get_full_url<R: DeserializeOwned>(
        &self,
        token: &str,
        full_url: &str,
    ) -> Result<R, Error> {
        self.get_with_url(token, full_url).await
    }

    /// Performs a GET request with custom headers and returns the resource as JSON.
    ///
    /// # Arguments
    ///
    /// * `token` - Authorization token
    /// * `url` - Resource path (e.g. "rooms")
    /// * `params` - Query parameters
    /// * `host` - Optional custom host prefix
    pub async fn get_with_params<R: DeserializeOwned, P: Serialize>(
        &self,
        token: &str,
        url: &str,
        params: &P,
        host: Option<&str>,
    ) -> Result<R, Error> {
        let host_prefix = self.get_host_prefix(host, "rest");
        let full_url = format!("{host_prefix}/{url}");
        self.request_with_query(
            "GET",
            AuthorizationType::Bearer(token),
            &full_url,
            Some(params),
            BODY_NONE,
        )
        .await
    }

    /// Performs a GET request.
    async fn get_with_url<R: DeserializeOwned>(
        &self,
        token: &str,
        full_url: &str,
    ) -> Result<R, Error> {
        self.request("GET", AuthorizationType::Bearer(token), full_url, BODY_NONE)
            .await
    }

    /// Performs a POST request with JSON body.
    pub async fn post<R: DeserializeOwned, D: Serialize>(
        &self,
        token: &str,
        url: &str,
        data: &D,
        host: Option<&str>,
    ) -> Result<R, Error> {
        let host_prefix = self.get_host_prefix(host, "rest");
        let full_url = format!("{host_prefix}/{url}");
        self.request(
            "POST",
            AuthorizationType::Bearer(token),
            &full_url,
            Some(Body::Json(data)),
        )
        .await
    }

    /// Performs a PUT request with JSON body.
    pub async fn put<R: DeserializeOwned, D: Serialize>(
        &self,
        token: &str,
        url: &str,
        data: &D,
        host: Option<&str>,
    ) -> Result<R, Error> {
        let host_prefix = self.get_host_prefix(host, "rest");
        let full_url = format!("{host_prefix}/{url}");
        self.request(
            "PUT",
            AuthorizationType::Bearer(token),
            &full_url,
            Some(Body::Json(data)),
        )
        .await
    }

    /// Performs a DELETE request.
    pub async fn delete<T: Gettable>(
        &self,
        token: &str,
        id: &str,
        host: Option<&str>,
    ) -> Result<(), Error> {
        let host_prefix = self.get_host_prefix(host, "rest");
        let full_url = format!("{host_prefix}/{}/{id}", T::API_ENDPOINT);
        let _: EmptyReply = self
            .request(
                "DELETE",
                AuthorizationType::Bearer(token),
                &full_url,
                BODY_NONE,
            )
            .await?;
        Ok(())
    }

    /// Performs a POST request with URL-encoded form body.
    /// Used primarily for OAuth authentication flows.
    pub async fn api_post_form_urlencoded<T: DeserializeOwned, B: Serialize>(
        &self,
        rest_method: &str,
        body: B,
        _params: Option<impl Serialize>,
        auth: AuthorizationType<'_>,
    ) -> Result<T, Error> {
        // Get the host prefix for the URL
        let url_trimmed = rest_method.split('?').next().unwrap_or(rest_method);
        let prefix = self
            .host_prefix
            .get(url_trimmed)
            .map_or(super::REST_HOST_PREFIX, String::as_str);
        let full_url = format!("{prefix}/{rest_method}");

        // params are not currently used but kept for API compatibility
        self.request("POST", auth, &full_url, Some(Body::UrlEncoded(body)))
            .await
    }

    /// Gets the host prefix for a given host key.
    fn get_host_prefix(&self, host: Option<&str>, default_key: &str) -> String {
        host.map_or_else(
            || {
                self.host_prefix
                    .get(default_key)
                    .map_or_else(|| super::REST_HOST_PREFIX.to_string(), Clone::clone)
            },
            ToString::to_string,
        )
    }

    /// Performs an HTTP request with query parameters and optional body.
    async fn request_with_query<R: DeserializeOwned, Q: Serialize, D: Serialize>(
        &self,
        method: &str,
        auth: AuthorizationType<'_>,
        url: &str,
        query: Option<&Q>,
        body: Option<Body<D>>,
    ) -> Result<R, Error> {
        trace!("{method} {url}");
        let mut req = self
            .web_client
            .request(method.parse().unwrap(), url)
            .header("User-Agent", format!("webex-rust/{}", super::CRATE_VERSION));

        // Add query parameters if provided
        if let Some(params) = query {
            req = req.query(params);
        }

        // Apply authorization
        req = match auth {
            AuthorizationType::None => req,
            AuthorizationType::Bearer(token) => req.bearer_auth(token),
            AuthorizationType::Basic { username, password } => {
                req.basic_auth(username, Some(password))
            }
        };

        req = match body {
            Some(Body::Json(data)) => req.json(&data),
            Some(Body::UrlEncoded(data)) => req.form(&data),
            None => req,
        };

        let response = req.send().await?;
        let status = response.status();
        let response_text = response.text().await?;

        if status.is_success() {
            trace!("Response: {response_text}");

            // Handle empty responses (like 204 No Content)
            if response_text.is_empty() {
                Ok(serde_json::from_str("{}")?)
            } else {
                Ok(serde_json::from_str(&response_text)?)
            }
        } else {
            // Log errors with appropriate level based on context
            Self::log_error(status, url, &response_text);
            Err(Self::handle_error_response(status, response_text))
        }
    }

    /// Performs an HTTP request with the given method, URL, and optional body.
    async fn request<R: DeserializeOwned, D: Serialize>(
        &self,
        method: &str,
        auth: AuthorizationType<'_>,
        url: &str,
        body: Option<Body<D>>,
    ) -> Result<R, Error> {
        trace!("{method} {url}");
        let mut req = self
            .web_client
            .request(method.parse().unwrap(), url)
            .header("User-Agent", format!("webex-rust/{}", super::CRATE_VERSION));

        // Apply authorization
        req = match auth {
            AuthorizationType::None => req,
            AuthorizationType::Bearer(token) => req.bearer_auth(token),
            AuthorizationType::Basic { username, password } => {
                req.basic_auth(username, Some(password))
            }
        };

        req = match body {
            Some(Body::Json(data)) => req.json(&data),
            Some(Body::UrlEncoded(data)) => req.form(&data),
            None => req,
        };

        let response = req.send().await?;
        let status = response.status();
        let response_text = response.text().await?;

        if status.is_success() {
            trace!("Response: {response_text}");

            // Handle empty responses (like 204 No Content)
            if response_text.is_empty() {
                Ok(serde_json::from_str("{}")?)
            } else {
                Ok(serde_json::from_str(&response_text)?)
            }
        } else {
            // Log errors with appropriate level based on context
            Self::log_error(status, url, &response_text);
            Err(Self::handle_error_response(status, response_text))
        }
    }

    /// Logs HTTP errors with appropriate log level based on context.
    fn log_error(status: StatusCode, url: &str, response_text: &str) {
        // Try to parse as JSON to get structured error message
        if let Ok(json_error) = serde_json::from_str::<serde_json::Value>(response_text) {
            if let Some(message) = json_error.get("message").and_then(|m| m.as_str()) {
                // Team 404 errors are expected when user doesn't have team access - log as debug
                if status == StatusCode::NOT_FOUND
                    && url.contains("/teams")
                    && message.contains("Could not find teams")
                {
                    trace!("HTTP {status} for {url}: {message} (expected when not a team member)");
                    return;
                }
            }
        }

        // Log all other errors at error level
        error!("HTTP {status}: {response_text}");
    }

    /// Handles error responses from the API.
    fn handle_error_response(status: StatusCode, response_text: String) -> Error {
        if response_text.starts_with("<!DOCTYPE html>") || response_text.starts_with("<html") {
            let title = extract_html_title(&response_text, status);
            Error::StatusText(status, title)
        } else {
            match serde_json::from_str::<serde_json::Value>(&response_text) {
                Ok(json) => {
                    if let Some(message) = json.get("message").and_then(|v| v.as_str()) {
                        Error::StatusText(status, message.to_string())
                    } else {
                        Error::StatusText(status, response_text)
                    }
                }
                Err(_) => Error::StatusText(status, response_text),
            }
        }
    }

    /// Generic GET request for any `Gettable` type.
    pub async fn get<T: Gettable + DeserializeOwned>(
        &self,
        token: &str,
        id: &str,
        host: Option<&str>,
    ) -> Result<T, Error> {
        let host_prefix = self.get_host_prefix(host, "rest");
        let full_url = format!("{host_prefix}/{}/{id}", T::API_ENDPOINT);
        self.get_with_url(token, &full_url).await
    }

    /// Generic LIST request for any `Gettable` type.
    pub async fn list<T: Gettable + DeserializeOwned>(
        &self,
        token: &str,
        params: &T::ListParams<'_>,
        host: Option<&str>,
    ) -> Result<Vec<T>, Error> {
        self.list_endpoint(token, T::API_ENDPOINT, Some(params), host)
            .await
    }

    pub(crate) async fn list_endpoint<T: DeserializeOwned, P: Serialize + ?Sized>(
        &self,
        token: &str,
        endpoint: &str,
        params: Option<&P>,
        host: Option<&str>,
    ) -> Result<Vec<T>, Error> {
        let host_prefix = host.map_or_else(
            || {
                self.host_prefix
                    .get(endpoint)
                    .cloned()
                    .or_else(|| self.host_prefix.get("rest").cloned())
                    .unwrap_or_else(|| super::REST_HOST_PREFIX.to_string())
            },
            ToString::to_string,
        );
        let full_url = format!("{host_prefix}/{endpoint}");
        self.paginated_get_list(AuthorizationType::Bearer(token), &full_url, params)
            .await
    }

    // Legacy API methods for compatibility with client/mod.rs

    /// Performs a GET request (legacy API name).
    pub async fn api_get<R: DeserializeOwned>(
        &self,
        rest_method: &str,
        params: Option<impl Serialize>,
        auth: AuthorizationType<'_>,
    ) -> Result<R, Error> {
        let url_trimmed = rest_method.split('?').next().unwrap_or(rest_method);
        let prefix = self
            .host_prefix
            .get(url_trimmed)
            .map_or(super::REST_HOST_PREFIX, String::as_str);
        let full_url = format!("{prefix}/{rest_method}");

        if let Some(params) = params {
            self.request_with_query("GET", auth, &full_url, Some(&params), BODY_NONE)
                .await
        } else {
            self.request("GET", auth, &full_url, BODY_NONE).await
        }
    }

    /// Performs a POST request with JSON body (legacy API name).
    pub async fn api_post<R: DeserializeOwned>(
        &self,
        rest_method: &str,
        body: impl Serialize,
        params: Option<impl Serialize>,
        auth: AuthorizationType<'_>,
    ) -> Result<R, Error> {
        let url_trimmed = rest_method.split('?').next().unwrap_or(rest_method);
        let prefix = self
            .host_prefix
            .get(url_trimmed)
            .map_or(super::REST_HOST_PREFIX, String::as_str);
        let full_url = format!("{prefix}/{rest_method}");

        if let Some(params) = params {
            // For params, we append them as query string
            let _params = params; // params need to be serialized to query string but we'll keep simple for now
        }

        self.request("POST", auth, &full_url, Some(Body::Json(body)))
            .await
    }

    /// Performs a PUT request with JSON body (legacy API name).
    pub async fn api_put<R: DeserializeOwned>(
        &self,
        rest_method: &str,
        body: impl Serialize,
        params: Option<impl Serialize>,
        auth: AuthorizationType<'_>,
    ) -> Result<R, Error> {
        let url_trimmed = rest_method.split('?').next().unwrap_or(rest_method);
        let prefix = self
            .host_prefix
            .get(url_trimmed)
            .map_or(super::REST_HOST_PREFIX, String::as_str);
        let full_url = format!("{prefix}/{rest_method}");

        if let Some(_params) = params {
            // params are not currently used but kept for API compatibility
        }

        self.request("PUT", auth, &full_url, Some(Body::Json(body)))
            .await
    }

    /// Performs a DELETE request (legacy API name).
    pub async fn api_delete(
        &self,
        rest_method: &str,
        params: Option<impl Serialize>,
        auth: AuthorizationType<'_>,
    ) -> Result<(), Error> {
        let url_trimmed = rest_method.split('?').next().unwrap_or(rest_method);
        let prefix = self
            .host_prefix
            .get(url_trimmed)
            .map_or(super::REST_HOST_PREFIX, String::as_str);
        let full_url = format!("{prefix}/{rest_method}");

        if let Some(_params) = params {
            // params are not currently used but kept for API compatibility
        }

        let _: EmptyReply = self.request("DELETE", auth, &full_url, BODY_NONE).await?;
        Ok(())
    }

    async fn paginated_get_list<T: DeserializeOwned, P: Serialize + ?Sized>(
        &self,
        auth: AuthorizationType<'_>,
        initial_url: &str,
        initial_query: Option<&P>,
    ) -> Result<Vec<T>, Error> {
        let mut next_url = Some(initial_url.to_string());
        let mut next_query = initial_query;
        let mut followed_pages = 0usize;
        let mut all_items = Vec::new();

        while let Some(current_url) = next_url.take() {
            trace!("GET {current_url}");
            let mut req = self
                .web_client
                .request(reqwest::Method::GET, &current_url)
                .header("User-Agent", format!("webex-rust/{}", super::CRATE_VERSION));

            if let Some(params) = next_query.take() {
                req = req.query(params);
            }

            req = match auth {
                AuthorizationType::None => req,
                AuthorizationType::Bearer(token) => req.bearer_auth(token),
                AuthorizationType::Basic { username, password } => {
                    req.basic_auth(username, Some(password))
                }
            };

            let response = req.send().await?;
            let status = response.status();
            let link_header = response
                .headers()
                .get(LINK)
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned);
            let response_text = response.text().await?;

            if !status.is_success() {
                Self::log_error(status, &current_url, &response_text);
                return Err(Self::handle_error_response(status, response_text));
            }

            trace!("Response: {response_text}");
            let list_result: ListResult<T> = if response_text.is_empty() {
                serde_json::from_str("{}")?
            } else {
                serde_json::from_str(&response_text)?
            };

            all_items.extend(list_result.items.or(list_result.devices).unwrap_or_default());

            next_url = link_header
                .as_deref()
                .and_then(Self::extract_next_link_url);

            if next_url.is_some() {
                if followed_pages >= MAX_NEXT_PAGE_REQUESTS {
                    return Err(Error::StatusText(
                        StatusCode::LOOP_DETECTED,
                        format!(
                            "Pagination safety cap of {MAX_NEXT_PAGE_REQUESTS} next pages reached"
                        ),
                    ));
                }
                followed_pages += 1;
            }
        }

        Ok(all_items)
    }

    fn extract_next_link_url(link_header: &str) -> Option<String> {
        let mut links = parse_with_rel(link_header).ok()?;
        links.remove("next").map(|link| link.raw_uri)
    }
}

impl Default for RestClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Extract title from HTML error page.
fn extract_html_title(html: &str, status: StatusCode) -> String {
    if let (Some(start_pos), Some(end_pos)) = (html.find("<title>"), html.find("</title>")) {
        let start = start_pos + 7;
        if start < end_pos && end_pos <= html.len() {
            return html[start..end_pos].to_string();
        }
    }
    format!("HTTP {} - HTML error page returned", status.as_u16())
}

#[cfg(test)]
#[allow(clippy::significant_drop_tightening)]
mod tests {
    use super::{RestClient, MAX_NEXT_PAGE_REQUESTS};
    use crate::types::room::{Room, RoomListParams};
    use mockito::Server;
    use reqwest::StatusCode;
    use serde_json::json;

    fn room_json(id: &str) -> serde_json::Value {
        json!({
            "id": id,
            "title": "Test Room",
            "type": "group",
            "isLocked": false,
            "lastActivity": "2024-01-01T00:00:00.000Z",
            "creatorId": "person-1",
            "created": "2024-01-01T00:00:00.000Z"
        })
    }

    #[tokio::test]
    async fn list_follows_only_rel_next_from_link_header() {
        let mut server = Server::new_async().await;
        let next_url = format!("{}/rooms?cursor=page-2", server.url());

        let page_one = server
            .mock("GET", "/rooms")
            .match_header("authorization", "Bearer test-token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_header(
                "link",
                &format!(
                    "<{}/rooms?cursor=prev>; rel=\"prev\", <{}>; rel=\"next\", <{}/rooms?cursor=last>; rel=\"last\"",
                    server.url(),
                    next_url,
                    server.url()
                ),
            )
            .with_body(json!({ "items": [room_json("room-1")] }).to_string())
            .create_async()
            .await;

        let page_two = server
            .mock("GET", "/rooms")
            .match_query(mockito::Matcher::UrlEncoded(
                "cursor".into(),
                "page-2".into(),
            ))
            .match_header("authorization", "Bearer test-token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(json!({ "items": [room_json("room-2")] }).to_string())
            .create_async()
            .await;

        let client = RestClient::new();
        let rooms = client
            .list::<Room>(
                "test-token",
                &RoomListParams::default(),
                Some(server.url().as_str()),
            )
            .await
            .unwrap();

        assert_eq!(rooms.len(), 2);
        assert_eq!(rooms[0].id, "room-1");
        assert_eq!(rooms[1].id, "room-2");
        page_one.assert_async().await;
        page_two.assert_async().await;
    }

    #[tokio::test]
    async fn list_ignores_malformed_link_header() {
        let mut server = Server::new_async().await;
        let page_one = server
            .mock("GET", "/rooms")
            .match_header("authorization", "Bearer test-token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_header("link", "garbage value")
            .with_body(json!({ "items": [room_json("room-1")] }).to_string())
            .create_async()
            .await;

        let client = RestClient::new();
        let rooms = client
            .list::<Room>(
                "test-token",
                &RoomListParams::default(),
                Some(server.url().as_str()),
            )
            .await
            .unwrap();

        assert_eq!(rooms.len(), 1);
        assert_eq!(rooms[0].id, "room-1");
        page_one.assert_async().await;
    }

    #[tokio::test]
    async fn list_stops_when_link_header_is_missing() {
        let mut server = Server::new_async().await;
        let page_one = server
            .mock("GET", "/rooms")
            .match_header("authorization", "Bearer test-token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(json!({ "items": [room_json("room-1")] }).to_string())
            .create_async()
            .await;

        let client = RestClient::new();
        let rooms = client
            .list::<Room>(
                "test-token",
                &RoomListParams::default(),
                Some(server.url().as_str()),
            )
            .await
            .unwrap();

        assert_eq!(rooms.len(), 1);
        assert_eq!(rooms[0].id, "room-1");
        page_one.assert_async().await;
    }

    #[tokio::test]
    async fn list_errors_when_pagination_cap_is_reached() {
        let mut server = Server::new_async().await;
        let mut mocks = Vec::new();

        for page in 0..=MAX_NEXT_PAGE_REQUESTS {
            let next_page = page + 1;
            let next_url = format!("{}/rooms?cursor=page-{next_page}", server.url());
            let body = json!({
                "items": [room_json(&format!("room-{page}"))]
            })
            .to_string();

            let mock = if page == 0 {
                server
                    .mock("GET", "/rooms")
                    .match_header("authorization", "Bearer test-token")
                    .with_status(200)
                    .with_header("content-type", "application/json")
                    .with_header("link", &format!("<{next_url}>; rel=\"next\""))
                    .with_body(body)
                    .create_async()
                    .await
            } else {
                server
                    .mock("GET", "/rooms")
                    .match_query(mockito::Matcher::UrlEncoded(
                        "cursor".into(),
                        format!("page-{page}"),
                    ))
                    .match_header("authorization", "Bearer test-token")
                    .with_status(200)
                    .with_header("content-type", "application/json")
                    .with_header("link", &format!("<{next_url}>; rel=\"next\""))
                    .with_body(body)
                    .create_async()
                    .await
            };
            mocks.push(mock);
        }

        let client = RestClient::new();
        let error = client
            .list::<Room>(
                "test-token",
                &RoomListParams::default(),
                Some(server.url().as_str()),
            )
            .await
            .unwrap_err();

        assert!(matches!(error, crate::error::Error::StatusText(StatusCode::LOOP_DETECTED, _)));
        assert!(error.to_string().contains("Pagination safety cap of 100 next pages reached"));

        for mock in mocks {
            mock.assert_async().await;
        }
    }
}
