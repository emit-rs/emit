compile_error!("unsupported target");

use std::{fmt, future::Future, sync::Arc, time::Duration};

use crate::{
    client::http::{HttpContent, HttpVersion},
    data::EncodedPayload,
    internal_metrics::InternalMetrics,
    Error,
};

pub(crate) struct HttpConnection {}

impl HttpConnection {
    pub fn new<F: Future<Output = Result<(), Error>> + Send + 'static>(
        _version: HttpVersion,
        _metrics: Arc<InternalMetrics>,
        _url: impl AsRef<str>,
        _allow_compression: bool,
        _headers: impl Into<Vec<(String, String)>>,
        _request: impl Fn(HttpContent) -> Result<HttpContent, Error> + Send + Sync + 'static,
        _response: impl Fn(HttpResponse) -> F + Send + Sync + 'static,
    ) -> Result<Self, Error> {
        unreachable!()
    }

    pub fn uri(&self) -> &HttpUri {
        unreachable!()
    }

    pub async fn send(&self, _body: EncodedPayload, _timeout: Duration) -> Result<(), Error> {
        unreachable!()
    }
}

pub(crate) struct HttpUri {}

impl fmt::Display for HttpUri {
    fn fmt(&self, _f: &mut fmt::Formatter) -> fmt::Result {
        unreachable!()
    }
}

impl HttpUri {
    pub fn is_https(&self) -> bool {
        unreachable!()
    }

    pub fn host(&self) -> &str {
        unreachable!()
    }

    pub fn authority(&self) -> &str {
        unreachable!()
    }

    pub fn port(&self) -> u16 {
        unreachable!()
    }
}

pub(crate) struct HttpResponse {}

impl HttpResponse {
    pub fn http_status(&self) -> u16 {
        todo!()
    }

    pub async fn stream_trailers(
        self,
        mut _trailer: impl FnMut(&str, &str),
    ) -> Result<(), Error> {
        todo!()
    }
}
