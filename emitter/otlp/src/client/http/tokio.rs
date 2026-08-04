/*!
HTTP transport based on `hyper` and `tokio`.

This transport supports HTTP1 and gRPC via HTTP2.
*/

use std::{
    fmt,
    future::Future,
    pin::Pin,
    sync::{Arc, LazyLock, Mutex},
    task::{Context, Poll},
    time::Duration,
};

use hyper::{
    Method, Request,
    body::{self, Body, Frame, SizeHint},
    client::conn::{http1, http2},
};

use crate::{
    Error,
    client::http::{
        ClientRequestSender, HttpContent, HttpContentCursor, HttpUri, HttpVersion,
        outgoing_traceparent_header,
    },
    data::EncodedPayload,
    internal_metrics::InternalMetrics,
    telemetry_sdk_name, telemetry_sdk_version,
};

static USER_AGENT: LazyLock<String> =
    LazyLock::new(|| format!("{}/{}", telemetry_sdk_name(), telemetry_sdk_version()));

async fn connect(
    metrics: &InternalMetrics,
    version: HttpVersion,
    uri: &HttpUri,
) -> Result<HttpSender, Error> {
    let io = tokio::net::TcpStream::connect((uri.host(), uri.port()))
        .await
        .map_err(|e| {
            metrics.transport_conn_failed.increment();

            Error::new("failed to connect TCP stream", e)
        })?;

    // Disable Nagle's algorithm; requests are written in several small pieces
    // (headers, framing, payload chunks), and coalescing them against delayed
    // ACKs stalls each request by tens of milliseconds
    let _ = io.set_nodelay(true);

    metrics.transport_conn_established.increment();

    if uri.is_https() {
        #[cfg(feature = "tls")]
        {
            let io = tls_handshake(metrics, io, uri, version).await?;

            http_handshake(metrics, version, io).await
        }
        #[cfg(not(feature = "tls"))]
        {
            return Err(Error::msg("https support requires the `tls` Cargo feature"));
        }
    } else {
        http_handshake(metrics, version, io).await
    }
}

#[cfg(feature = "tls")]
fn alpn_protocol(version: HttpVersion) -> &'static str {
    // HTTP2 is commonly negotiated via ALPN during the TLS handshake
    // We don't support protocol downgrades, so only advertise exactly the protocol
    // we expect to communicate on
    match version {
        HttpVersion::Http1 => "http/1.1",
        HttpVersion::Http2 => "h2",
    }
}

/**
TLS using the native platform
*/
#[cfg(all(feature = "tls", feature = "tls-native"))]
async fn tls_handshake(
    metrics: &InternalMetrics,
    io: tokio::net::TcpStream,
    uri: &HttpUri,
    version: HttpVersion,
) -> Result<impl tokio::io::AsyncRead + tokio::io::AsyncWrite + Send + Sync + Unpin + 'static, Error>
{
    use tokio_native_tls::{TlsConnector, native_tls};

    let domain = uri.host();

    let connector = TlsConnector::from(
        native_tls::TlsConnector::builder()
            .request_alpns(&[alpn_protocol(version)])
            .build()
            .map_err(|e| {
                metrics.transport_conn_tls_failed.increment();

                Error::new("failed to create TLS connector", e)
            })?,
    );

    let io = connector.connect(domain, io).await.map_err(|e| {
        metrics.transport_conn_tls_failed.increment();

        Error::new("failed to perform TLS handshake", e)
    })?;

    metrics.transport_conn_tls_handshake.increment();

    Ok(io)
}

/**
TLS using `rustls`
*/
#[cfg(all(feature = "tls", not(feature = "tls-native")))]
async fn tls_handshake(
    metrics: &InternalMetrics,
    io: tokio::net::TcpStream,
    uri: &HttpUri,
    version: HttpVersion,
) -> Result<impl tokio::io::AsyncRead + tokio::io::AsyncWrite + Send + Sync + Unpin + 'static, Error>
{
    use tokio_rustls::TlsConnector;

    let domain = uri.host().to_owned().try_into().map_err(|e| {
        metrics.transport_conn_tls_failed.increment();

        Error::new(format_args!("could not extract a DNS name from {uri}"), e)
    })?;

    let conn = TlsConnector::from(tls_client_config(metrics, version));

    let io = conn.connect(domain, io).await.map_err(|e| {
        metrics.transport_conn_tls_failed.increment();

        Error::new("failed to connect TLS stream", e)
    })?;

    metrics.transport_conn_tls_handshake.increment();

    Ok(io)
}

/**
Get the shared `rustls` configuration for connections speaking `version`.
*/
#[cfg(all(feature = "tls", not(feature = "tls-native")))]
fn tls_client_config(
    metrics: &InternalMetrics,
    version: HttpVersion,
) -> Arc<tokio_rustls::rustls::ClientConfig> {
    use std::sync::OnceLock;

    use tokio_rustls::rustls;

    // Cache and re-use configuration across connections; it's expensive to produce
    static HTTP1: OnceLock<Arc<rustls::ClientConfig>> = OnceLock::new();
    static HTTP2: OnceLock<Arc<rustls::ClientConfig>> = OnceLock::new();

    let slot = match version {
        HttpVersion::Http1 => &HTTP1,
        HttpVersion::Http2 => &HTTP2,
    };

    slot.get_or_init(|| {
        let mut root_store = rustls::RootCertStore::empty();

        let certs = rustls_native_certs::load_native_certs();

        if !certs.errors.is_empty() {
            metrics.transport_conn_tls_failed.increment();

            for err in certs.errors {
                emit::warn!(rt: emit::runtime::internal(), "failed to load native certificate: {err}");
            }
        }

        for cert in certs.certs {
            let _ = root_store.add(cert);
        }

        let mut config = rustls::ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();

        config.alpn_protocols = vec![alpn_protocol(version).into()];

        Arc::new(config)
    })
    .clone()
}

async fn http_handshake(
    metrics: &InternalMetrics,
    version: HttpVersion,
    io: impl tokio::io::AsyncRead + tokio::io::AsyncWrite + Send + Sync + Unpin + 'static,
) -> Result<HttpSender, Error> {
    match version {
        HttpVersion::Http1 => http1_handshake(metrics, io).await,
        HttpVersion::Http2 => http2_handshake(metrics, io).await,
    }
}

async fn http1_handshake(
    metrics: &InternalMetrics,
    io: impl tokio::io::AsyncRead + tokio::io::AsyncWrite + Send + Sync + Unpin + 'static,
) -> Result<HttpSender, Error> {
    let (sender, conn) = http1::handshake(HttpIo(io)).await.map_err(|e| {
        metrics.transport_conn_failed.increment();

        Error::new("failed to perform HTTP1 handshake", e)
    })?;

    tokio::task::spawn(async move {
        let _ = conn.await;
    });

    Ok(HttpSender::Http1(sender))
}

async fn http2_handshake(
    metrics: &InternalMetrics,
    io: impl tokio::io::AsyncRead + tokio::io::AsyncWrite + Send + Sync + Unpin + 'static,
) -> Result<HttpSender, Error> {
    let (sender, conn) = http2::handshake(TokioAmbientExecutor, HttpIo(io))
        .await
        .map_err(|e| {
            metrics.transport_conn_failed.increment();

            Error::new("failed to perform HTTP2 handshake", e)
        })?;

    tokio::task::spawn(async move {
        let _ = conn.await;
    });

    Ok(HttpSender::Http2(sender))
}

async fn send_request<'a>(
    metrics: &'a InternalMetrics,
    sender: &mut HttpSender,
    uri: &'a HttpUri,
    headers: impl Iterator<Item = (&'a str, &'a str)>,
    content: HttpContent,
) -> Result<HttpResponse, Error> {
    let res = sender
        .send_request(
            metrics,
            http_request(metrics, sender.version(), uri, headers, content)?,
        )
        .await?;

    Ok(res)
}

fn http_request<'a>(
    metrics: &'a InternalMetrics,
    version: HttpVersion,
    uri: &'a HttpUri,
    headers: impl Iterator<Item = (&'a str, &'a str)>,
    content: HttpContent,
) -> Result<Request<HttpContent>, Error> {
    let request_uri = match version {
        // HTTP1 requests to origin servers carry an origin-form target
        // with the authority in the host header
        HttpVersion::Http1 => http::Uri::builder()
            .path_and_query(
                uri.0
                    .path_and_query()
                    .cloned()
                    .unwrap_or_else(|| http::uri::PathAndQuery::from_static("/")),
            )
            .build()
            .map_err(|e| Error::new("failed to construct request target", e))?,
        // HTTP2 carries the full URI in its pseudo headers
        HttpVersion::Http2 => uri.0.clone(),
    };

    let mut req = Request::builder().uri(request_uri).method(Method::POST);

    for (k, v) in content.custom_headers {
        req = req.header(*k, *v);
    }

    req = req.header("host", uri.authority());

    for (name, value) in content.iter_headers() {
        req = req.header(name, &*value);
    }

    for (k, v) in headers {
        req = req.header(k, v);
    }

    // These values don't override custom headers
    req = req.header("user-agent", &*USER_AGENT);

    // Propagate traceparent for the batch
    req = if let Some((k, v)) = outgoing_traceparent_header() {
        req.header(k, v)
    } else {
        req
    };

    Ok(req.body(content).map_err(|e| {
        metrics.transport_request_failed.increment();

        Error::new("failed to stream HTTP body", e)
    })?)
}

pub(crate) struct HttpConnection {
    metrics: Arc<InternalMetrics>,
    version: HttpVersion,
    allow_compression: bool,
    uri: HttpUri,
    headers: Vec<(String, String)>,
    request: Box<dyn Fn(HttpContent) -> Result<HttpContent, Error> + Send + Sync>,
    response: Box<
        dyn Fn(HttpResponse) -> Pin<Box<dyn Future<Output = Result<(), Error>> + Send>>
            + Send
            + Sync,
    >,
    sender: Mutex<Option<HttpSender>>,
}

pub(crate) struct HttpResponse {
    res: hyper::Response<body::Incoming>,
}

impl HttpConnection {
    pub(crate) fn new<F: Future<Output = Result<(), Error>> + Send + 'static>(
        version: HttpVersion,
        metrics: Arc<InternalMetrics>,
        url: impl AsRef<str>,
        allow_compression: bool,
        headers: impl Into<Vec<(String, String)>>,
        request: impl Fn(HttpContent) -> Result<HttpContent, Error> + Send + Sync + 'static,
        response: impl Fn(HttpResponse) -> F + Send + Sync + 'static,
    ) -> Result<Self, Error> {
        let url = url.as_ref();

        Ok(HttpConnection {
            uri: HttpUri::new(url)?,
            version,
            allow_compression,
            request: Box::new(request),
            response: Box::new(move |res| Box::pin(response(res))),
            headers: headers.into(),
            sender: Mutex::new(None),
            metrics,
        })
    }

    fn poison(&self) -> Option<HttpSender> {
        self.sender.lock().unwrap().take()
    }

    fn unpoison(&self, sender: HttpSender) {
        *self.sender.lock().unwrap() = Some(sender);
    }

    fn uri(&self) -> &HttpUri {
        &self.uri
    }

    async fn send(&self, body: EncodedPayload, timeout: Duration) -> Result<(), Error> {
        let res = tokio::time::timeout(timeout, async {
            let mut sender = match self.poison() {
                // Only re-use the previous connection if it's still open; servers
                // and load balancers regularly close idle or long-lived connections
                Some(sender) if !sender.is_closed() => sender,
                _ => connect(&self.metrics, self.version, &self.uri).await?,
            };

            let body =
                HttpContent::new(self.allow_compression, &self.request, &self.metrics, body)?;

            let res = send_request(
                &self.metrics,
                &mut sender,
                &self.uri,
                self.headers.iter().map(|(k, v)| (&**k, &**v)),
                body,
            )
            .await?;

            self.unpoison(sender);

            (self.response)(res).await
        })
        .await
        .map_err(|e| Error::new("failed to send request within its timeout", e))?;

        res
    }
}

impl ClientRequestSender for HttpConnection {
    fn uri(&self) -> &(impl fmt::Display + 'static) {
        self.uri()
    }

    fn send(
        &self,
        body: EncodedPayload,
        timeout: Duration,
    ) -> impl Future<Output = Result<(), Error>> {
        self.send(body, timeout)
    }
}

enum HttpSender {
    Http1(http1::SendRequest<HttpContent>),
    Http2(http2::SendRequest<HttpContent>),
}

impl HttpSender {
    fn version(&self) -> HttpVersion {
        match self {
            HttpSender::Http1(_) => HttpVersion::Http1,
            HttpSender::Http2(_) => HttpVersion::Http2,
        }
    }

    fn is_closed(&self) -> bool {
        match self {
            HttpSender::Http1(sender) => sender.is_closed(),
            HttpSender::Http2(sender) => sender.is_closed(),
        }
    }

    async fn send_request(
        &mut self,
        metrics: &InternalMetrics,
        req: Request<HttpContent>,
    ) -> Result<HttpResponse, Error> {
        let res = match self {
            HttpSender::Http1(sender) => sender.send_request(req).await,
            HttpSender::Http2(sender) => sender.send_request(req).await,
        }
        .map_err(|e| {
            metrics.transport_request_failed.increment();

            Error::new("failed to send HTTP request", e)
        })?;

        metrics.transport_request_sent.increment();

        Ok(HttpResponse { res })
    }
}

impl Body for HttpContent {
    type Data = HttpContentCursor;

    type Error = std::convert::Infallible;

    fn poll_frame(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let unpinned = self.get_mut();

        Poll::Ready(
            unpinned
                .next_content_cursor()
                .map(|cursor| Ok(Frame::data(cursor))),
        )
    }

    fn is_end_stream(&self) -> bool {
        !self.has_next_content_cursor()
    }

    fn size_hint(&self) -> SizeHint {
        SizeHint::with_exact(self.content_len() as u64)
    }
}

impl HttpResponse {
    pub fn http_status(&self) -> u16 {
        self.res.status().as_u16()
    }

    pub fn header(&self, name: &str) -> Option<&str> {
        self.res.headers().get(name).and_then(|v| v.to_str().ok())
    }

    pub async fn drain(self) -> Result<(), Error> {
        // NOTE: Reading trailers requires reading the body too
        self.stream_trailers(|_, _| {}).await
    }

    pub async fn stream_trailers(
        mut self,
        mut trailer: impl FnMut(&str, &str),
    ) -> Result<(), Error> {
        struct BufNext<'a, T>(&'a mut body::Incoming, &'a mut T);

        impl<'a, T: FnMut(&str, &str)> Future for BufNext<'a, T> {
            type Output = Result<bool, Error>;

            fn poll(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Self::Output> {
                // SAFETY: `self` does not use interior pinning
                let BufNext(incoming, trailer) = unsafe { Pin::get_unchecked_mut(self) };

                match Pin::new(incoming).poll_frame(ctx) {
                    Poll::Ready(Some(Ok(frame))) => {
                        if let Some(trailers) = frame.trailers_ref() {
                            for (k, v) in trailers {
                                let k = k.as_str();

                                if let Ok(v) = v.to_str() {
                                    (trailer)(k, v)
                                }
                            }
                        }

                        Poll::Ready(Ok(true))
                    }
                    Poll::Ready(None) => Poll::Ready(Ok(false)),
                    Poll::Ready(Some(Err(e))) => {
                        Poll::Ready(Err(Error::new("failed to read HTTP response body", e)))
                    }
                    Poll::Pending => Poll::Pending,
                }
            }
        }

        let frame = self.res.body_mut();

        while BufNext(frame, &mut trailer).await? {}

        Ok(())
    }
}

struct HttpIo<T>(T);

impl<T: tokio::io::AsyncRead> hyper::rt::Read for HttpIo<T> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        mut buf: hyper::rt::ReadBufCursor<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        // SAFETY: `io` inherits the pinning requirements of `self`
        let io = unsafe { self.map_unchecked_mut(|io| &mut io.0) };

        // SAFETY: `io` does not uninitialize any bytes
        let mut read_buf = tokio::io::ReadBuf::uninit(unsafe { buf.as_mut() });

        match tokio::io::AsyncRead::poll_read(io, cx, &mut read_buf) {
            Poll::Ready(Ok(())) => {
                let read = read_buf.filled().len();

                // SAFETY: The bytes being advanced have been initialized by `read_buf`
                unsafe { buf.advance(read) };

                Poll::Ready(Ok(()))
            }
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<T: tokio::io::AsyncWrite> hyper::rt::Write for HttpIo<T> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        // SAFETY: `io` inherits the pinning requirements of `self`
        let io = unsafe { self.map_unchecked_mut(|io| &mut io.0) };

        tokio::io::AsyncWrite::poll_write(io, cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), std::io::Error>> {
        // SAFETY: `io` inherits the pinning requirements of `self`
        let io = unsafe { self.map_unchecked_mut(|io| &mut io.0) };

        tokio::io::AsyncWrite::poll_flush(io, cx)
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        // SAFETY: `io` inherits the pinning requirements of `self`
        let io = unsafe { self.map_unchecked_mut(|io| &mut io.0) };

        tokio::io::AsyncWrite::poll_shutdown(io, cx)
    }
}

#[derive(Clone, Copy)]
struct TokioAmbientExecutor;

impl<F: Future + Send + 'static> hyper::rt::Executor<F> for TokioAmbientExecutor
where
    F::Output: Send + 'static,
{
    fn execute(&self, fut: F) {
        tokio::spawn(fut);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::data::{Json, RawEncoder};

    #[test]
    fn http1_request_target_is_origin_form() {
        let metrics = InternalMetrics::default();
        let uri = HttpUri::new("http://localhost:4718/v1/logs").unwrap();
        let content =
            HttpContent::new(false, |content| Ok(content), &metrics, Json::encode(42)).unwrap();

        let req = http_request(
            &metrics,
            HttpVersion::Http1,
            &uri,
            ([] as [(&str, &str); 0]).into_iter(),
            content,
        )
        .unwrap();

        assert_eq!("/v1/logs", req.uri().to_string());
        assert_eq!("localhost:4718", req.headers()["host"]);
    }

    #[test]
    fn http2_request_target_is_absolute() {
        let metrics = InternalMetrics::default();
        let uri = HttpUri::new("http://localhost:4718/v1/logs").unwrap();
        let content =
            HttpContent::new(false, |content| Ok(content), &metrics, Json::encode(42)).unwrap();

        let req = http_request(
            &metrics,
            HttpVersion::Http2,
            &uri,
            ([] as [(&str, &str); 0]).into_iter(),
            content,
        )
        .unwrap();

        assert_eq!("http://localhost:4718/v1/logs", req.uri().to_string());
    }

    #[test]
    fn default_http_port_is_80() {
        let uri = HttpUri("http://example.com".parse().unwrap());
        assert_eq!(80, uri.port());
    }

    #[test]
    fn default_https_port_is_443() {
        let uri = HttpUri("https://example.com".parse().unwrap());
        assert_eq!(443, uri.port());
    }

    #[test]
    fn default_user_agent() {
        let metrics = InternalMetrics::default();
        let uri = HttpUri::new("http://localhost:4718").unwrap();
        let headers = [] as [(&str, &str); 0];
        let content =
            HttpContent::new(false, |content| Ok(content), &metrics, Json::encode(42)).unwrap();

        let req = http_request(
            &metrics,
            HttpVersion::Http1,
            &uri,
            headers.into_iter(),
            content,
        )
        .unwrap();

        let agent = req.headers().get("user-agent").unwrap().to_str().unwrap();

        assert_eq!(&*USER_AGENT, agent);
    }

    #[test]
    fn custom_user_agent() {
        let metrics = InternalMetrics::default();
        let uri = HttpUri::new("http://localhost:4718").unwrap();
        let headers = [("user-agent", "custom-agent")];
        let content =
            HttpContent::new(false, |content| Ok(content), &metrics, Json::encode(42)).unwrap();

        let req = http_request(
            &metrics,
            HttpVersion::Http1,
            &uri,
            headers.into_iter(),
            content,
        )
        .unwrap();

        let agent = req.headers().get("user-agent").unwrap().to_str().unwrap();

        assert_eq!("custom-agent", agent);
    }
}
