#[cfg(not(all(
    target_arch = "wasm32",
    target_vendor = "unknown",
    target_os = "unknown"
)))]
#[path = "http/tokio.rs"]
mod imp;

#[cfg(all(
    target_arch = "wasm32",
    target_vendor = "unknown",
    target_os = "unknown"
))]
#[path = "http/web.rs"]
mod imp;

pub(crate) use self::imp::*;

use std::io::Cursor;

use bytes::Buf;

use crate::{
    client::Encoding,
    data::{EncodedPayload, PreEncodedCursor},
    Error,
};

#[derive(Clone)]
pub(crate) struct HttpContent {
    custom_headers: &'static [(&'static str, &'static str)],
    content_frame: Option<HttpContentHeader>,
    content_payload: Option<HttpContentPayload>,
    content_type_header: &'static str,
    content_encoding_header: Option<&'static str>,
}

fn content_type_of(payload: &EncodedPayload) -> &'static str {
    match Encoding::of(payload) {
        Encoding::Proto => "application/x-protobuf",
        Encoding::Json => "application/json",
    }
}

impl HttpContent {
    fn raw(payload: EncodedPayload) -> Self {
        HttpContent {
            content_frame: None,
            content_type_header: content_type_of(&payload),
            content_encoding_header: None,
            custom_headers: &[],
            content_payload: Some(HttpContentPayload::PreEncoded(payload)),
        }
    }

    #[cfg(feature = "gzip")]
    fn gzip(payload: EncodedPayload) -> Result<Self, Error> {
        use std::io::Write as _;

        let content_type = content_type_of(&payload);

        let mut enc = flate2::write::GzEncoder::new(
            Vec::with_capacity(payload.len()),
            flate2::Compression::fast(),
        );

        let mut payload = payload.into_cursor();
        loop {
            let chunk = payload.chunk();
            if chunk.len() == 0 {
                break;
            }

            enc.write_all(chunk)
                .map_err(|e| Error::new("failed to compress a chunk of bytes", e))?;
            payload.advance(chunk.len());
        }

        let buf = enc
            .finish()
            .map_err(|e| Error::new("failed to finalize compression", e))?;

        Ok(HttpContent {
            content_type_header: content_type,
            content_encoding_header: Some("gzip"),
            custom_headers: &[],
            content_frame: None,
            content_payload: Some(HttpContentPayload::Bytes(buf.into_boxed_slice())),
        })
    }

    pub fn with_content_frame(mut self, header: [u8; 5]) -> Self {
        self.content_frame = Some(HttpContentHeader::SmallBytes(header));
        self
    }

    pub fn content_type_header(&self) -> &'static str {
        self.content_type_header
    }

    pub fn with_content_type_header(mut self, content_type: &'static str) -> Self {
        self.content_type_header = content_type;
        self
    }

    pub fn take_content_encoding_header(&mut self) -> Option<&'static str> {
        self.content_encoding_header.take()
    }

    pub fn with_headers(mut self, headers: &'static [(&'static str, &'static str)]) -> Self {
        self.custom_headers = headers;
        self
    }

    pub fn content_len(&self) -> usize {
        self.content_frame_len() + self.content_payload_len()
    }

    pub fn content_frame_len(&self) -> usize {
        self.content_frame
            .as_ref()
            .map(|header| header.len())
            .unwrap_or(0)
    }

    pub fn content_payload_len(&self) -> usize {
        self.content_payload
            .as_ref()
            .map(|payload| payload.len())
            .unwrap_or(0)
    }
}

#[derive(Clone)]
enum HttpContentHeader {
    // NOTE: Basically hardcodes gRPC header, but could be generalized if it was worth it
    SmallBytes([u8; 5]),
}

#[derive(Clone)]
enum HttpContentPayload {
    PreEncoded(EncodedPayload),
    #[allow(dead_code)]
    Bytes(Box<[u8]>),
}

impl HttpContentHeader {
    fn len(&self) -> usize {
        match self {
            HttpContentHeader::SmallBytes(header) => header.len(),
        }
    }

    fn into_cursor(self) -> HttpContentCursor {
        match self {
            HttpContentHeader::SmallBytes(header) => {
                HttpContentCursor::SmallBytes(Cursor::new(header))
            }
        }
    }
}

impl HttpContentPayload {
    fn len(&self) -> usize {
        match self {
            HttpContentPayload::PreEncoded(payload) => payload.len(),
            HttpContentPayload::Bytes(payload) => payload.len(),
        }
    }

    fn into_cursor(self) -> HttpContentCursor {
        match self {
            HttpContentPayload::PreEncoded(payload) => {
                HttpContentCursor::PreEncoded(payload.into_cursor())
            }
            HttpContentPayload::Bytes(payload) => HttpContentCursor::Bytes(Cursor::new(payload)),
        }
    }
}

pub(crate) enum HttpContentCursor {
    PreEncoded(PreEncodedCursor),
    Bytes(Cursor<Box<[u8]>>),
    SmallBytes(Cursor<[u8; 5]>),
}

impl Buf for HttpContentCursor {
    fn remaining(&self) -> usize {
        match self {
            HttpContentCursor::PreEncoded(buf) => buf.remaining(),
            HttpContentCursor::Bytes(buf) => buf.remaining(),
            HttpContentCursor::SmallBytes(buf) => buf.remaining(),
        }
    }

    fn chunk(&self) -> &[u8] {
        match self {
            HttpContentCursor::PreEncoded(buf) => buf.chunk(),
            HttpContentCursor::Bytes(buf) => buf.chunk(),
            HttpContentCursor::SmallBytes(buf) => buf.chunk(),
        }
    }

    fn advance(&mut self, cnt: usize) {
        match self {
            HttpContentCursor::PreEncoded(buf) => buf.advance(cnt),
            HttpContentCursor::Bytes(buf) => buf.advance(cnt),
            HttpContentCursor::SmallBytes(buf) => buf.advance(cnt),
        }
    }
}
