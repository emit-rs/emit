/*!
HTTP transport based on `hyper` and `tokio`.

This transport supports HTTP1 and gRPC via HTTP2.
*/

use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, LazyLock, Mutex},
    task::{self, Context, Poll},
    time::Duration,
};

use hyper::{
    body::{self, Body, Frame, SizeHint},
    client::conn::{http1, http2},
    Method, Request,
};

use crate::{
    client::http::{
        outgoing_traceparent_header, HttpContent, HttpContentCursor, HttpUri, HttpVersion,
    },
    data::EncodedPayload,
    internal_metrics::InternalMetrics,
    telemetry_sdk_name, telemetry_sdk_version, Error,
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

    metrics.transport_conn_established.increment();

    if uri.is_https() {
        #[cfg(feature = "tls")]
        {
            let io = tls_handshake(metrics, io, uri).await?;

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

/*
TLS using the native platform
*/
#[cfg(all(feature = "tls", feature = "tls-native"))]
async fn tls_handshake(
    metrics: &InternalMetrics,
    io: tokio::net::TcpStream,
    uri: &HttpUri,
) -> Result<impl tokio::io::AsyncRead + tokio::io::AsyncWrite + Send + Sync + Unpin + 'static, Error>
{
    use tokio_native_tls::{native_tls, TlsConnector};

    let domain = uri.host();

    let connector = TlsConnector::from(native_tls::TlsConnector::new().map_err(|e| {
        metrics.transport_conn_tls_failed.increment();

        Error::new("failed to create TLS connector", e)
    })?);

    let io = connector.connect(domain, io).await.map_err(|e| {
        metrics.transport_conn_tls_failed.increment();

        Error::new("failed to perform TLS handshake", e)
    })?;

    metrics.transport_conn_tls_handshake.increment();

    Ok(io)
}

/*
TLS using `rustls`
*/
#[cfg(all(feature = "tls", not(feature = "tls-native")))]
async fn tls_handshake(
    metrics: &InternalMetrics,
    io: tokio::net::TcpStream,
    uri: &HttpUri,
) -> Result<impl tokio::io::AsyncRead + tokio::io::AsyncWrite + Send + Sync + Unpin + 'static, Error>
{
    use tokio_rustls::{rustls, TlsConnector};

    let domain = uri.host().to_owned().try_into().map_err(|e| {
        metrics.transport_conn_tls_failed.increment();

        Error::new(format_args!("could not extract a DNS name from {uri}"), e)
    })?;

    let tls = {
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

        Arc::new(
            rustls::ClientConfig::builder()
                .with_root_certificates(root_store)
                .with_no_client_auth(),
        )
    };

    let conn = TlsConnector::from(tls);

    let io = conn.connect(domain, io).await.map_err(|e| {
        metrics.transport_conn_tls_failed.increment();

        Error::new("failed to connect TLS stream", e)
    })?;

    metrics.transport_conn_tls_handshake.increment();

    Ok(io)
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
        .send_request(metrics, http_request(metrics, uri, headers, content)?)
        .await?;

    Ok(res)
}

fn http_request<'a>(
    metrics: &'a InternalMetrics,
    uri: &'a HttpUri,
    headers: impl Iterator<Item = (&'a str, &'a str)>,
    content: HttpContent,
) -> Result<Request<HttpContent>, Error> {
    let mut req = Request::builder().uri(&uri.0).method(Method::POST);

    for (k, v) in content.custom_headers {
        req = req.header(*k, *v);
    }

    req = req.header("host", uri.authority());
    req = req.header("user-agent", &*USER_AGENT);

    for (name, value) in content.iter_headers() {
        req = req.header(name, &*value);
    }

    for (k, v) in headers {
        req = req.header(k, v);
    }

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
    pub fn new<F: Future<Output = Result<(), Error>> + Send + 'static>(
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

    pub fn uri(&self) -> &HttpUri {
        &self.uri
    }

    pub async fn send(&self, body: EncodedPayload, timeout: Duration) -> Result<(), Error> {
        let res = tokio::time::timeout(timeout, async {
            let mut sender = match self.poison() {
                Some(sender) => sender,
                None => connect(&self.metrics, self.version, &self.uri).await?,
            };

            let body = HttpContent::new(
                self.allow_compression,
                &self.uri,
                &self.request,
                &self.metrics,
                body,
            )?;

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

enum HttpSender {
    Http1(http1::SendRequest<HttpContent>),
    Http2(http2::SendRequest<HttpContent>),
}

impl HttpSender {
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

    pub async fn stream_trailers(
        mut self,
        mut trailer: impl FnMut(&str, &str),
    ) -> Result<(), Error> {
        struct BufNext<'a, T>(&'a mut body::Incoming, &'a mut T);

        impl<'a, T: FnMut(&str, &str)> Future for BufNext<'a, T> {
            type Output = Result<bool, Error>;

            fn poll(self: Pin<&mut Self>, ctx: &mut task::Context<'_>) -> task::Poll<Self::Output> {
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
}
