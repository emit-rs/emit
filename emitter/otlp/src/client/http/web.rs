/*!
HTTP Transport based on `fetch`.

This transport supports HTTP1.
*/

use std::{borrow::Cow, error, fmt, future::Future, pin::Pin, sync::Arc, time::Duration};

use bytes::Buf;
use js_sys::{Map, Object, Reflect, Uint8Array, JSON};
use wasm_bindgen::prelude::*;

use crate::{
    client::http::{outgoing_traceparent_header, HttpContent, HttpUri, HttpVersion},
    data::EncodedPayload,
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
            return Err(Error::msg(
                "only HTTP1-based transports are supported by fetch",
            ));
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

        // NOTE: Headers added here may affect CORS
        let headers = js_headers(
            content
                .iter_headers()
                .chain(
                    self.headers
                        .iter()
                        .map(|(k, v)| (&**k, Cow::Borrowed(&**v))),
                )
                .chain(outgoing_traceparent_header().map(|(k, v)| (k, Cow::Owned(v)))),
        );

        let body = js_body(content)?;

        let signal = js_signal(timeout);

        let res = fetch(
            resource,
            &JsValue::from(js_fetch_init("POST", &headers, &body, signal.as_ref())),
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
        Err(Error::msg("streaming trailers is not supported by fetch"))
    }
}

fn js_status(res: &JsValue) -> Result<u16, Error> {
    Ok(Reflect::get(&res, &JsValue::from("status"))
        .map_err(|e| Error::new("failed to read fetch response", JsError::new(e)))?
        .as_f64()
        .ok_or_else(|| Error::msg("the fetch response status is not a number"))? as u16)
}

fn js_fetch_init(
    method: &str,
    headers: &Map,
    body: &Uint8Array,
    signal: Option<&AbortSignal>,
) -> Object {
    let result = Object::new();

    Reflect::set(&result, &JsValue::from("method"), &JsValue::from(method))
        .expect("failed to set fetch init field");
    Reflect::set(&result, &JsValue::from("headers"), &JsValue::from(headers))
        .expect("failed to set fetch init field");
    Reflect::set(&result, &JsValue::from("body"), &JsValue::from(body))
        .expect("failed to set fetch init field");

    if let Some(signal) = signal {
        Reflect::set(&result, &JsValue::from("signal"), &JsValue::from(signal))
            .expect("failed to set fetch init field");
    }

    result
}

fn js_headers(headers: impl Iterator<Item = (impl AsRef<str>, impl AsRef<str>)>) -> Map {
    let result = Map::new();

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

    let result = Uint8Array::new_with_length(length);

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

fn js_signal(timeout: Duration) -> Option<AbortSignal> {
    // `AbortSignal.timeout` is fairly new (mid 2024), so only call it if it's defined
    ABORT_SIGNAL.with(|signal| {
        if Reflect::get(signal, &JsValue::from("timeout"))
            .ok()
            .unwrap_or(JsValue::UNDEFINED)
            .is_undefined()
        {
            return None;
        }

        Some(AbortSignal::timeout_millis(timeout.as_millis() as u32))
    })
}

#[derive(Debug)]
struct JsError {
    msg: String,
    payload: Vec<(String, String)>,
}

impl fmt::Display for JsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.msg)?;

        if self.payload.len() > 0 {
            f.write_str(" (")?;

            let mut first = true;
            for (k, v) in &self.payload {
                if !first {
                    write!(f, ", {k}: {v}")?;
                } else {
                    write!(f, "{k}: {v}")?;
                }

                first = false;
            }

            f.write_str(")")?;
        }

        Ok(())
    }
}

impl error::Error for JsError {}

impl JsError {
    fn new(err: JsValue) -> Self {
        if let Some(msg) = err.as_string() {
            return JsError {
                msg,
                payload: Vec::new(),
            };
        }

        let err = Object::from(err);

        let msg = err.to_string().into();

        let mut payload = Vec::new();
        extract_err(&err, &mut 0, &mut String::from("err"), &mut payload);

        JsError { msg, payload }
    }
}

fn extract_err(
    obj: &Object,
    depth: &mut usize,
    path: &mut String,
    payload: &mut Vec<(String, String)>,
) {
    for key in Object::keys(obj) {
        if let Ok(value) = Reflect::get(obj, &key) {
            let original_len = path.len();
            path.push('.');
            path.push_str(&js_stringify(&key));
            *depth += 1;

            if value.is_object() && *depth < 6 {
                extract_err(&Object::from(value), depth, path, payload);
            } else {
                payload.push((path.clone(), js_stringify(&value)));
            }

            *depth -= 1;
            path.truncate(original_len);
        }
    }
}

fn js_stringify(value: &JsValue) -> String {
    if let Some(value) = value.as_string() {
        value
    } else if let Ok(value) = JSON::stringify(&value) {
        value.into()
    } else {
        String::from("<unknown>")
    }
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = "fetch", catch)]
    async fn fetch(uri: String, init: &JsValue) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(thread_local_v2, js_name = "AbortSignal")]
    static ABORT_SIGNAL: JsValue;

    #[wasm_bindgen]
    type AbortSignal;

    #[wasm_bindgen(static_method_of = AbortSignal, js_name = "timeout")]
    fn timeout_millis(ms: u32) -> AbortSignal;
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
    fn abort_signal_is_defined() {
        assert!(js_signal(Duration::from_millis(1)).is_some());
    }
}
