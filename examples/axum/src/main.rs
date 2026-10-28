use axum::{
    routing::get,
    Router,
};

#[tokio::main]
async fn main() {
    // build our application with a single route
    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .layer(emit_axum::EmitLayer);

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

mod emit_axum {
    use std::pin::Pin;
    use std::task::{Context, Poll};
    use axum::extract::Request;
    use axum::routing::Route;
    use tower::{Layer, Service};

    // TODO: `emit_traceparent::setup()`

    #[derive(Debug, Clone)]
    pub struct EmitLayer;

    impl Layer<Route> for EmitLayer {
        type Service = EmitService<Route>;

        fn layer(&self, inner: Route) -> Self::Service {
            EmitService(inner)
        }
    }

    #[derive(Debug, Clone)]
    pub struct EmitService<S>(S);

    impl<S: Service<Request>> Service<Request> for EmitService<S> {
        type Response = S::Response;
        type Error = S::Error;
        type Future = EmitFuture<S::Future>;

        fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            self.0.poll_ready(cx)
        }

        fn call(&mut self, req: Request) -> Self::Future {
            // TODO: Get an `emit::Frame` based on the request
            EmitFuture(self.0.call(req))
        }
    }

    pub struct EmitFuture<F>(F);

    impl<F: Future> Future for EmitFuture<F> {
        type Output = F::Output;

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            // SAFETY: `EmitFuture` projects pinning to its fields
            let inner = unsafe {
                self.map_unchecked_mut(|pinned| &mut pinned.0)
            };

            inner.poll(cx)
        }
    }
}
