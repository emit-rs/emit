/*!
HTTP Transport based on `fetch`.

This transport supports HTTP1.
*/

use std::{borrow::Cow, error, fmt, future::Future, io::Read, pin::Pin, sync::Arc, time::Duration};

use bytes::Buf;
use js_sys::{Map, Object, Promise, Reflect, Uint8Array};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

use crate::{
    client::http::{HttpContent, HttpUri, HttpVersion},
    data::{EncodedPayload, PreEncodedCursor},
    internal_metrics::InternalMetrics,
    Error,
};

pub(crate) struct HttpConnection {
    uri: HttpUri,
    allow_compression: bool,
    headers: Vec<(String, String)>,
    request: Box<dyn Fn(HttpContent) -> Result<HttpContent, Error> + Send + Sync>,
    response: Box<
        dyn Fn(HttpResponse) -> Pin<Box<dyn Future<Output = Result<(), Error>> + Send>>
            + Send
            + Sync,
    >,
    metrics: Arc<InternalMetrics>,
}

impl HttpConnection {
    pub fn new<F: Future<Output = Result<(), Error>> + Send + 'static>(
        version: HttpVersion,
        metrics: Arc<InternalMetrics>,
        url: impl AsRef<str>,
        allow_compression: bool,
        headers: impl Into<Vec<(String, String)>>,
        request: impl Fn(HttpContent) -> Result<HttpContent, Error> + Send + Sync + 'static,
        response: impl Fn(HttpResponse) -> F + Send + Sync + 'static,
    ) -> Result<Self, Error> {
        if version != HttpVersion::Http1 {
            unimplemented!()
        }

        Ok(HttpConnection {
            uri: HttpUri::new(url)?,
            allow_compression,
            request: Box::new(request),
            response: Box::new(move |res| Box::pin(response(res))),
            headers: headers.into(),
            metrics,
        })
    }

    pub fn uri(&self) -> &HttpUri {
        &self.uri
    }

    pub async fn send(&self, body: EncodedPayload, timeout: Duration) -> Result<(), Error> {
        let resource = self.uri.to_string();

        let content = HttpContent::new(
            self.allow_compression,
            &self.uri,
            &self.request,
            &self.metrics,
            body,
        )?;

        let headers = js_headers(
            content.iter_headers().chain(
                self.headers
                    .iter()
                    .map(|(k, v)| (&**k, Cow::Borrowed(&**v))),
            ),
        );

        let body = js_body(content)?;

        let res = fetch(
            self.uri.to_string(),
            &JsValue::from(fs_fetch_init("POST", headers, body)),
        )
        .await
        .map_err(|e| Error::new("failed to send fetch request", JsError::new(e)))?;

        (self.response)(HttpResponse {
            status: js_status(&res)?,
        })
        .await
    }
}

pub(crate) struct HttpResponse {
    status: u16,
}

impl HttpResponse {
    pub fn http_status(&self) -> u16 {
        self.status
    }

    pub async fn stream_trailers(self, _trailer: impl FnMut(&str, &str)) -> Result<(), Error> {
        unimplemented!()
    }
}

fn js_status(res: &JsValue) -> Result<u16, Error> {
    Ok(Reflect::get(&res, &JsValue::from("status"))
        .map_err(|e| Error::new("failed to read fetch response", JsError::new(e)))?
        .as_f64()
        .ok_or_else(|| Error::msg("the fetch response status is not a number"))? as u16)
}

fn fs_fetch_init(method: &str, headers: Map, body: Uint8Array) -> Object {
    let mut result = Object::new();

    Reflect::set(&result, &JsValue::from("method"), &JsValue::from(method))
        .expect("failed to set fetch init field");
    Reflect::set(&result, &JsValue::from("headers"), &JsValue::from(headers))
        .expect("failed to set fetch init field");
    Reflect::set(&result, &JsValue::from("body"), &JsValue::from(body))
        .expect("failed to set fetch init field");

    result
}

fn js_headers(headers: impl Iterator<Item = (impl AsRef<str>, impl AsRef<str>)>) -> Map {
    let mut result = Map::new();

    for (k, v) in headers {
        let name = JsValue::from(k.as_ref());
        let value = JsValue::from(v.as_ref());

        result.set(&name, &value);
    }

    result
}

fn js_body(mut body: HttpContent) -> Result<Uint8Array, Error> {
    let length = body
        .content_len()
        .try_into()
        .map_err(|e| Error::new("fetch request body is too large", e))?;

    let mut result = Uint8Array::new_with_length(length);

    let mut offset = 0;
    while let Some(mut body) = body.next_content_cursor() {
        while body.remaining() > 0 {
            let chunk = body.chunk();

            // SAFETY: The view is valid for the duration of `set`, which copies elements
            unsafe { result.set(&Uint8Array::view(chunk), offset as u32) };

            offset += chunk.len();
            body.advance(chunk.len());
        }
    }

    Ok(result)
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
extern "C" {
    #[wasm_bindgen(js_name = "fetch", catch)]
    async fn fetch(uri: String, init: &JsValue) -> Result<JsValue, JsValue>;
}
