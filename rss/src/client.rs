use std::error::Error;
use std::fmt;

use generic_async_http_client::Request;
use ouroboros::self_referencing;
use roxmltree_serde::{from_doc, Document, ParseError as XmlParseError};

use crate::feed::Feed;

#[derive(Debug)]
pub struct RssRequest {
    req: Request,
}

impl RssRequest {
    pub fn new(url: &str) -> Result<Self, RssError> {
        let req = Request::new("GET", url).map_err(RssError::Http)?;
        Ok(Self { req })
    }

    pub async fn exec<'a>(self) -> Result<RssResponse, RssError> {
        let mut resp = self.req.exec().await.map_err(RssError::Http)?;

        if (200..300).contains(&resp.status_code()) {
            let body = resp.text().await.map_err(RssError::Http)?;
            RssResponseTryBuilder {
                body: body.into_boxed_str(),
                document_builder: |str| Document::parse(str).map_err(RssError::XmlParse),
                feed_builder: |doc| from_doc(doc).map_err(RssError::XmlDecode),
            }
            .try_build()
        } else {
            Err(RssError::UnexpectedResponse(resp.status_code()))
        }
    }
}

#[self_referencing]
pub struct RssResponse {
    body: Box<str>,
    #[borrows(body)]
    #[covariant]
    document: Document<'this>,
    #[borrows(document)]
    #[covariant]
    pub feed: Feed<'this>,
}

#[derive(Debug)]
pub enum RssError {
    Http(generic_async_http_client::Error),
    UnexpectedResponse(u16),
    XmlDecode(roxmltree_serde::Error),
    XmlParse(XmlParseError),
}

impl fmt::Display for RssError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RssError::Http(err) => write!(f, "http error: {err}"),
            RssError::UnexpectedResponse(code) => write!(f, "unexpected response code {code}"),
            RssError::XmlDecode(err) => write!(f, "xml decode error: {err}"),
            RssError::XmlParse(err) => write!(f, "xml parse error: {err}"),
        }
    }
}

impl Error for RssError {}
