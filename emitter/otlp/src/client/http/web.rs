/*!
HTTP Transport based on `fetch`.

This transport supports HTTP1.
*/

use std::{borrow::Cow, error, fmt, future::Future, io::Read, sync::Arc, time::Duration};

use bytes::Buf;
use js_sys::{Map, Object, Promise, Uint8Array};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

use crate::{
    client::http::HttpContent,
    data::{EncodedPayload, PreEncodedCursor},
    internal_metrics::InternalMetrics,
    Error,
};

pub(crate) struct HttpConnection {
    uri: HttpUri,
    allow_compression: bool,
    headers: Vec<(String, String)>,
    request: Box<dyn Fn(HttpContent) -> Result<HttpContent, Error> + Send + Sync>,
    metrics: Arc<InternalMetrics>,
}

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
        &self.uri
    }

    pub async fn send(&self, body: EncodedPayload, timeout: Duration) -> Result<Vec<u8>, Error> {
        let resource = self.uri.to_string();

        let content = HttpContent::new(
            self.allow_compression,
            &self.uri,
            &self.request,
            &self.metrics,
            body,
        )?;

        let init = FetchInit {
            headers: http_headers(
                content.iter_headers().chain(
                    self.headers
                        .iter()
                        .map(|(k, v)| (&**k, Cow::Borrowed(&**v))),
                ),
            ),
            body: http_body(content)?,
        };

        fetch(self.uri.to_string(), init)
            .await
            .map_err(|e| Error::new("failed to send fetch request", JsError::new(e)))?;

        todo!()
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum HttpVersion {
    Http1,
    Http2,
}

pub(crate) struct HttpUri(http::Uri);

impl fmt::Display for HttpUri {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl HttpUri {
    pub fn is_https(&self) -> bool {
        self.0.scheme() == Some(&http::uri::Scheme::HTTPS)
    }

    pub fn host(&self) -> &str {
        self.0.host().expect("invalid URI")
    }

    pub fn authority(&self) -> &str {
        self.0.authority().expect("invalid URI").as_ref()
    }

    pub fn port(&self) -> u16 {
        self.0
            .port_u16()
            .unwrap_or_else(|| if self.is_https() { 443 } else { 80 })
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

struct JsError(String);

impl fmt::Debug for JsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

impl fmt::Display for JsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl error::Error for JsError {}

impl JsError {
    fn new(err: JsValue) -> Self {
        if let Some(err) = err.as_string() {
            return JsError(err);
        }

        let err = Object::from(err);

        JsError(err.to_string().into())
    }
}

#[wasm_bindgen]
pub struct FetchInit {
    body: Uint8Array,
    headers: Map,
}

fn http_body(mut body: HttpContent) -> Result<Uint8Array, Error> {
    let length = body
        .content_len()
        .try_into()
        .map_err(|e| Error::new("the body content cannot be converted into a Uint8Array", e))?;

    let mut buf = Uint8Array::new_with_length(length);

    let mut offset = 0;
    while let Some(mut body) = body.next_content_cursor() {
        while body.remaining() > 0 {
            let chunk = body.chunk();

            // SAFETY: The view is valid for the duration of `set`, which copies elements
            unsafe { buf.set(&Uint8Array::view(chunk), offset as u32) };

            offset += chunk.len();
            body.advance(chunk.len());
        }
    }

    Ok(buf)
}

fn http_headers(headers: impl Iterator<Item = (impl AsRef<str>, impl AsRef<str>)>) -> Map {
    let mut buf = Map::new();

    for (k, v) in headers {
        let name = JsValue::from(k.as_ref());
        let value = JsValue::from(v.as_ref());

        buf.set(&name, &value);
    }

    buf
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = "fetch", catch)]
    async fn fetch(uri: String, init: FetchInit) -> Result<JsValue, JsValue>;
}
