//! Prometheus metrics collection
//!
//! This module provides comprehensive metrics collection using Prometheus
//! for monitoring application performance, health, and business metrics.

use once_cell::sync::Lazy;
use prometheus::{
    register_histogram_vec, register_int_counter_vec, register_int_gauge_vec, HistogramVec,
    IntCounterVec, IntGaugeVec, Registry, TextEncoder, Encoder,
};
use std::time::Instant;

/// Global metrics registry
pub static METRICS_REGISTRY: Lazy<Registry> = Lazy::new(Registry::new);

/// HTTP request counter
pub static HTTP_REQUESTS_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "http_requests_total",
        "Total number of HTTP requests",
        &["method", "path", "status"]
    )
    .expect("Failed to create HTTP requests counter")
});

/// HTTP request duration histogram
pub static HTTP_REQUEST_DURATION: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "http_request_duration_seconds",
        "HTTP request duration in seconds",
        &["method", "path"],
        vec![0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]
    )
    .expect("Failed to create HTTP request duration histogram")
});

/// Database query counter
pub static DB_QUERIES_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "db_queries_total",
        "Total number of database queries",
        &["operation", "status"]
    )
    .expect("Failed to create database queries counter")
});

/// Database query duration histogram
pub static DB_QUERY_DURATION: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "db_query_duration_seconds",
        "Database query duration in seconds",
        &["operation"],
        vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0]
    )
    .expect("Failed to create database query duration histogram")
});

/// Cache operations counter
pub static CACHE_OPERATIONS_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "cache_operations_total",
        "Total number of cache operations",
        &["operation", "result"]
    )
    .expect("Failed to create cache operations counter")
});

/// Cache hit rate gauge
pub static CACHE_HIT_RATE: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "cache_hit_rate",
        "Cache hit rate",
        &["cache_type"],
        vec![0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 0.95, 0.99, 1.0]
    )
    .expect("Failed to create cache hit rate histogram")
});

/// Active database connections
pub static DB_CONNECTIONS_ACTIVE: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "db_connections_active",
        "Number of active database connections",
        &["pool"]
    )
    .expect("Failed to create active database connections gauge")
});

/// Idle database connections
pub static DB_CONNECTIONS_IDLE: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "db_connections_idle",
        "Number of idle database connections",
        &["pool"]
    )
    .expect("Failed to create idle database connections gauge")
});

/// Asset registry operations counter
pub static ASSET_OPERATIONS_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "asset_operations_total",
        "Total number of asset operations",
        &["operation", "status"]
    )
    .expect("Failed to create asset operations counter")
});

/// Total assets gauge
pub static ASSETS_TOTAL: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "assets_total",
        "Total number of assets in registry",
        &["status"]
    )
    .expect("Failed to create assets total gauge")
});

/// Event publishing counter
pub static EVENTS_PUBLISHED_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "events_published_total",
        "Total number of events published",
        &["event_type", "destination", "status"]
    )
    .expect("Failed to create events published counter")
});

/// Registry information gauge (version)
pub static REGISTRY_INFO: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "registry_info",
        "Registry information",
        &["version", "build"]
    )
    .expect("Failed to create registry info gauge")
});

/// Initialize metrics
pub fn init_metrics() {
    // Force initialization of all lazy metrics
    Lazy::force(&HTTP_REQUESTS_TOTAL);
    Lazy::force(&HTTP_REQUEST_DURATION);
    Lazy::force(&DB_QUERIES_TOTAL);
    Lazy::force(&DB_QUERY_DURATION);
    Lazy::force(&CACHE_OPERATIONS_TOTAL);
    Lazy::force(&CACHE_HIT_RATE);
    Lazy::force(&DB_CONNECTIONS_ACTIVE);
    Lazy::force(&DB_CONNECTIONS_IDLE);
    Lazy::force(&ASSET_OPERATIONS_TOTAL);
    Lazy::force(&ASSETS_TOTAL);
    Lazy::force(&EVENTS_PUBLISHED_TOTAL);
    Lazy::force(&REGISTRY_INFO);

    // Set registry info
    REGISTRY_INFO
        .with_label_values(&[env!("CARGO_PKG_VERSION"), "unknown"])
        .set(1);

    tracing::info!("Metrics initialized successfully");
}

/// Render metrics in Prometheus text format
pub fn render_metrics() -> Result<String, String> {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();

    let mut buffer = Vec::new();
    encoder
        .encode(&metric_families, &mut buffer)
        .map_err(|e| format!("Failed to encode metrics: {}", e))?;

    String::from_utf8(buffer).map_err(|e| format!("Failed to convert metrics to string: {}", e))
}

/// Timer for measuring operation duration
pub struct MetricsTimer {
    start: Instant,
    histogram: HistogramVec,
    labels: Vec<String>,
}

impl MetricsTimer {
    /// Create a new timer
    pub fn new(histogram: HistogramVec, labels: Vec<String>) -> Self {
        Self {
            start: Instant::now(),
            histogram,
            labels,
        }
    }

    /// Stop the timer and record the duration
    pub fn stop(self) {
        let duration = self.start.elapsed();
        let label_refs: Vec<&str> = self.labels.iter().map(|s| s.as_str()).collect();

        self.histogram
            .with_label_values(&label_refs)
            .observe(duration.as_secs_f64());
    }
}

/// Record HTTP request metrics
pub fn record_http_request(method: &str, path: &str, status: u16, duration_secs: f64) {
    HTTP_REQUESTS_TOTAL
        .with_label_values(&[method, path, &status.to_string()])
        .inc();

    HTTP_REQUEST_DURATION
        .with_label_values(&[method, path])
        .observe(duration_secs);
}

/// Record database query metrics
pub fn record_db_query(operation: &str, success: bool, duration_secs: f64) {
    let status = if success { "success" } else { "error" };

    DB_QUERIES_TOTAL
        .with_label_values(&[operation, status])
        .inc();

    DB_QUERY_DURATION
        .with_label_values(&[operation])
        .observe(duration_secs);
}

/// Record cache operation
pub fn record_cache_operation(operation: &str, result: &str) {
    CACHE_OPERATIONS_TOTAL
        .with_label_values(&[operation, result])
        .inc();
}

/// Record asset operation
pub fn record_asset_operation(operation: &str, success: bool) {
    let status = if success { "success" } else { "error" };

    ASSET_OPERATIONS_TOTAL
        .with_label_values(&[operation, status])
        .inc();
}

/// Record event publication
pub fn record_event_published(event_type: &str, destination: &str, success: bool) {
    let status = if success { "success" } else { "error" };

    EVENTS_PUBLISHED_TOTAL
        .with_label_values(&[event_type, destination, status])
        .inc();
}

/// Update database connection pool metrics
pub fn update_db_pool_metrics(pool_name: &str, active: usize, idle: usize) {
    DB_CONNECTIONS_ACTIVE
        .with_label_values(&[pool_name])
        .set(active as i64);

    DB_CONNECTIONS_IDLE
        .with_label_values(&[pool_name])
        .set(idle as i64);
}

/// Update total assets metric
pub fn update_assets_total(status: &str, count: i64) {
    ASSETS_TOTAL
        .with_label_values(&[status])
        .set(count);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_initialization() {
        init_metrics();

        // Verify metrics can be rendered
        let metrics = render_metrics().expect("Failed to render metrics");
        assert!(metrics.contains("registry_info"));
    }

    #[test]
    fn test_record_http_request() {
        record_http_request("GET", "/api/v1/assets", 200, 0.123);

        let metrics = render_metrics().expect("Failed to render metrics");
        assert!(metrics.contains("http_requests_total"));
    }

    #[test]
    fn test_record_db_query() {
        record_db_query("select", true, 0.050);

        let metrics = render_metrics().expect("Failed to render metrics");
        assert!(metrics.contains("db_queries_total"));
    }

    #[test]
    fn test_metrics_timer() {
        let timer = MetricsTimer::new(
            HTTP_REQUEST_DURATION.clone(),
            vec!["GET".to_string(), "/test".to_string()],
        );

        std::thread::sleep(std::time::Duration::from_millis(10));
        timer.stop();

        let metrics = render_metrics().expect("Failed to render metrics");
        assert!(metrics.contains("http_request_duration_seconds"));
    }
}
