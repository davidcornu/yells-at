use crate::BoxedError;
use hyper::{self, client::HttpConnector, Body, Request};
use hyper_rustls::{HttpsConnector, HttpsConnectorBuilder};
use image::{self, ImageFormat};
use scraper::{Html, Selector};
use url::Url;

const USER_AGENT: &str = "yells.at (@davidcornu)";

pub type HttpClient = hyper::Client<HttpsConnector<HttpConnector>>;

pub fn build_http_client() -> HttpClient {
    let connector = HttpsConnectorBuilder::new()
        .with_native_roots()
        .https_only()
        .enable_http2()
        .build();

    hyper::Client::builder().build::<_, Body>(connector)
}

pub struct Client<'a> {
    http_client: &'a HttpClient,
}

impl<'a> Client<'a> {
    pub fn new(http_client: &'a HttpClient) -> Self {
        Client { http_client }
    }

    fn public_profile_url(username: &str) -> Url {
        let mut profile_url: Url = "https://github.com".parse().unwrap();
        profile_url.path_segments_mut().unwrap().extend(&[username]);
        profile_url
    }

    async fn avatar_url(&self, username: &str) -> Result<Option<Url>, BoxedError> {
        let req = Request::builder()
            .method("GET")
            .uri(Self::public_profile_url(username).as_str())
            .header("Accept", "text/html")
            .header("User-Agent", USER_AGENT)
            .body(Body::empty())?;

        let res = self.http_client.request(req).await?;

        if !res.status().is_success() {
            return Ok(None);
        }

        let body = hyper::body::to_bytes(res.into_body()).await?;
        let document = Html::parse_document(&String::from_utf8_lossy(&body));
        let selector = Selector::parse("meta[property='og:image']").unwrap();

        Ok(document
            .select(&selector)
            .nth(0)
            .and_then(|element_ref| element_ref.value().attr("content"))
            .and_then(|content| content.parse::<Url>().ok()))
    }

    fn image_format_from_response<T>(res: &hyper::Response<T>) -> Option<ImageFormat> {
        res.headers()
            .get("content-type")
            .and_then(|header| match header.to_str() {
                Ok("image/png") => Some(ImageFormat::Png),
                Ok("image/jpeg") => Some(ImageFormat::Jpeg),
                _ => None,
            })
    }

    pub async fn fetch_avatar(
        &self,
        email: &str,
    ) -> Result<Option<image::DynamicImage>, BoxedError> {
        let addr = match self.avatar_url(email).await? {
            Some(addr) => addr,
            None => return Ok(None),
        };

        let req = Request::builder()
            .method("GET")
            .uri(addr.as_str())
            .header("User-Agent", USER_AGENT)
            .body(Body::empty())
            .unwrap();

        let res = self.http_client.request(req).await?;

        if !res.status().is_success() {
            return Ok(None);
        }

        let format = match Self::image_format_from_response(&res) {
            Some(format) => format,
            None => return Ok(None),
        };

        let bytes = hyper::body::to_bytes(res.into_body()).await?;
        let image = tokio::task::block_in_place(|| {
            image::load_from_memory_with_format(&bytes, format)
        })?;

        Ok(Some(image))
    }
}
