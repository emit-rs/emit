/*!
HTTP Transport based on `fetch`.

This transport supports HTTP1.
*/

use std::{fmt, future::Future, sync::Arc, time::Duration};

use crate::{
    client::http::HttpContent, data::EncodedPayload, internal_metrics::InternalMetrics, Error,
};

pub(crate) struct HttpConnection {}

impl HttpConnection {
    pub fn new<F: Future<Output = Result<Vec<u8>, Error>> + Send + 'static>(
        version: HttpVersion,
        metrics: Arc<InternalMetrics>,
        url: impl AsRef<str>,
        allow_compression: bool,
        headers: impl Into<Vec<(String, String)>>,
        request: impl Fn(HttpContent) -> Result<HttpContent, Error> + Send + Sync + 'static,
        response: impl Fn(HttpResponse) -> F + Send + Sync + 'static,
    ) -> Result<Self, Error> {
        todo!()
    }

    pub fn uri(&self) -> &HttpUri {
        todo!()
    }

    pub async fn send(&self, body: EncodedPayload, timeout: Duration) -> Result<Vec<u8>, Error> {
        todo!()
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum HttpVersion {
    Http1,
    Http2,
}

pub(crate) struct HttpUri {}

impl fmt::Display for HttpUri {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        todo!()
    }
}

impl HttpUri {
    pub fn is_https(&self) -> bool {
        todo!()
    }

    pub fn host(&self) -> &str {
        todo!()
    }

    pub fn authority(&self) -> &str {
        todo!()
    }

    pub fn port(&self) -> u16 {
        todo!()
    }
}

pub(crate) struct HttpResponse {}

impl HttpResponse {
    pub fn http_status(&self) -> u16 {
        todo!()
    }

    pub async fn stream_payload(
        mut self,
        mut body: impl FnMut(&[u8]),
        mut trailer: impl FnMut(&str, &str),
    ) -> Result<(), Error> {
        todo!()
    }
}
