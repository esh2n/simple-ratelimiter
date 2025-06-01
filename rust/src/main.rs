use axum::{
    extract::{ConnectInfo, Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde::Serialize;
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tower_http::cors::CorsLayer;

mod algorithms;
use algorithms::{
    fixed_window_counter::FixedWindowCounter, leaky_bucket::LeakyBucket, sliding_window_counter::SlidingWindowCounter,
    sliding_window_log::SlidingWindowLog, token_bucket::TokenBucket, RateLimiter,
};

type RateLimiters = Arc<HashMap<String, Arc<dyn RateLimiter>>>;

#[derive(Serialize)]
struct DataResponse {
    message: String,
    algorithm: String,
    timestamp: String,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    retry_after: Option<i64>,
}

#[derive(Serialize)]
struct StatsResponse {
    algorithm: String,
    stats: algorithms::RateLimiterStats,
    available_algorithms: Vec<String>,
}

#[derive(Serialize)]
struct AlgorithmsResponse {
    algorithms: Vec<String>,
    endpoints: HashMap<String, String>,
}

async fn data_handler(
    Path(algorithm): Path<String>,
    State(limiters): State<RateLimiters>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    mut headers: HeaderMap,
) -> impl IntoResponse {
    let limiter = match limiters.get(&algorithm) {
        Some(l) => l,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "Algorithm not found".to_string(),
                    retry_after: None,
                }),
            )
                .into_response();
        }
    };

    let client_id = addr.ip().to_string();
    let key = format!("{}:{}", algorithm, client_id);

    let allowed = limiter.allow(&key);
    let stats = limiter.get_stats(&key);

    headers.insert("X-RateLimit-Limit", stats.limit.to_string().parse().unwrap());
    headers.insert(
        "X-RateLimit-Remaining",
        stats.remaining.to_string().parse().unwrap(),
    );
    headers.insert(
        "X-RateLimit-Reset",
        stats.reset_at.to_rfc3339().parse().unwrap(),
    );

    if !allowed {
        let retry_after = (stats.reset_at - chrono::Utc::now()).num_seconds().max(0);
        headers.insert("Retry-After", retry_after.to_string().parse().unwrap());

        return (
            StatusCode::TOO_MANY_REQUESTS,
            headers,
            Json(ErrorResponse {
                error: "Too Many Requests".to_string(),
                retry_after: Some(retry_after),
            }),
        )
            .into_response();
    }

    (
        StatusCode::OK,
        headers,
        Json(DataResponse {
            message: "Success".to_string(),
            algorithm,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }),
    )
        .into_response()
}

async fn stats_handler(
    Path(algorithm): Path<String>,
    State(limiters): State<RateLimiters>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    let limiter = match limiters.get(&algorithm) {
        Some(l) => l,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "Algorithm not found".to_string(),
                    retry_after: None,
                }),
            )
                .into_response();
        }
    };

    let client_id = addr.ip().to_string();
    let key = format!("{}:{}", algorithm, client_id);
    let stats = limiter.get_stats(&key);

    let available_algorithms: Vec<String> = limiters.keys().cloned().collect();

    (
        StatusCode::OK,
        Json(StatsResponse {
            algorithm,
            stats,
            available_algorithms,
        }),
    )
        .into_response()
}

async fn algorithms_handler(State(limiters): State<RateLimiters>) -> impl IntoResponse {
    let algorithms: Vec<String> = limiters.keys().cloned().collect();
    let mut endpoints = HashMap::new();
    endpoints.insert("data".to_string(), "/api/data/:algorithm".to_string());
    endpoints.insert("stats".to_string(), "/api/stats/:algorithm".to_string());

    Json(AlgorithmsResponse {
        algorithms,
        endpoints,
    })
}

#[tokio::main]
async fn main() {
    let mut rate_limiters: HashMap<String, Arc<dyn RateLimiter>> = HashMap::new();
    
    rate_limiters.insert(
        "token-bucket".to_string(),
        Arc::new(TokenBucket::new(10, 1000, 1.0)),
    );
    rate_limiters.insert(
        "leaky-bucket".to_string(),
        Arc::new(LeakyBucket::new(10, 10.0)),
    );
    rate_limiters.insert(
        "fixed-window".to_string(),
        Arc::new(FixedWindowCounter::new(10, 60000)),
    );
    rate_limiters.insert(
        "sliding-window-log".to_string(),
        Arc::new(SlidingWindowLog::new(10, 60000)),
    );
    rate_limiters.insert(
        "sliding-window-counter".to_string(),
        Arc::new(SlidingWindowCounter::new(10, 60000)),
    );

    let rate_limiters = Arc::new(rate_limiters);

    let app = Router::new()
        .route("/api/data/:algorithm", get(data_handler))
        .route("/api/stats/:algorithm", get(stats_handler))
        .route("/api/algorithms", get(algorithms_handler))
        .layer(CorsLayer::permissive())
        .with_state(rate_limiters.clone());

    let port = std::env::var("PORT").unwrap_or_else(|_| "3002".to_string());
    let addr = format!("0.0.0.0:{}", port).parse::<SocketAddr>().unwrap();

    println!("Rust Rate Limiter Server running on {}", addr);
    println!(
        "Available algorithms: {}",
        rate_limiters.keys().cloned().collect::<Vec<_>>().join(", ")
    );

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}