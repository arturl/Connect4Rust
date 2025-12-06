use axum::{
    extract::Query,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use connect4::{best_move, MoveRequest, MoveResponse};
use tokio::net::TcpListener;
use tower_http::{cors::CorsLayer, services::ServeDir, trace::TraceLayer};
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();
    let app = app_router();

    let addr = "0.0.0.0:3000";
    let listener = TcpListener::bind(addr).await?;
    info!("Listening on http://{addr}");
    axum::serve(listener, app).await?;
    Ok(())
}

fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("info,tower_http=debug")
        .try_init();
}

fn app_router() -> Router {
    let api = Router::new().route("/move", get(handle_move));
    let spa = Router::new().nest_service(
        "/",
        ServeDir::new("web/dist").append_index_html_on_directories(true),
    );
    Router::new()
        .nest("/api", api)
        .merge(spa)
        .layer(
            CorsLayer::new()
                .allow_methods([axum::http::Method::GET])
                .allow_origin(axum::http::HeaderValue::from_static("*"))
                .allow_headers([header::CONTENT_TYPE]),
        )
        .layer(TraceLayer::new_for_http())
}

#[derive(Debug, serde::Deserialize)]
struct MoveQuery {
    position: String,
    level: u8,
}

async fn handle_move(Query(query): Query<MoveQuery>) -> Result<impl IntoResponse, ApiError> {
    let req = MoveRequest {
        position: query.position,
        level: query.level,
    };
    let mv = best_move(req)?;
    let headers = [(header::CACHE_CONTROL, "no-store")];
    Ok((headers, Json(mv)))
}

#[derive(Debug)]
struct ApiError(anyhow::Error);

impl<E: Into<anyhow::Error>> From<E> for ApiError {
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = StatusCode::BAD_REQUEST;
        let body = format!("{}", self.0);
        (status, body).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use axum::http::Request;
    use tower::util::ServiceExt;

    #[tokio::test]
    async fn http_move_endpoint() {
        let app = app_router();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/move?position=R4B4R5B5R6&level=4")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let mv: MoveResponse = serde_json::from_slice(&bytes).unwrap();
        assert!(mv.column < 7);
    }
}
