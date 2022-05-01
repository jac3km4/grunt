use generic_async_http_client::Request;
use ownref::{BoxOwned, BoxOwnedA};
use strong_xml::XmlRead;
use thiserror::Error;

use crate::feed::Feed;

#[derive(Error, Debug)]
pub enum RssError {
    #[error("http error: {0}")]
    HttpError(#[from] generic_async_http_client::Error),
    #[error("xml error: {0}")]
    XmlError(#[from] strong_xml::XmlError),
}

#[derive(Debug)]
pub struct RssRequest {
    req: Request,
}

impl RssRequest {
    pub fn new(url: &str) -> Result<Self, RssError> {
        let req = Request::new("GET", url)?;
        Ok(Self { req })
    }

    pub async fn exec<'a>(self) -> Result<OwnedFeed<'a>, RssError> {
        let body = self.req.exec().await?.text().await?;
        let res = BoxOwned::from_box(body.into_boxed_str()).try_map(|str| Feed::from_str(str))?;
        Ok(res)
    }
}

pub type OwnedFeed<'a> = BoxOwnedA<'a, str, Feed<'a>>;
