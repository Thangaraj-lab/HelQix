//! Request metrics in the Prometheus text format, with no extra dependency.
//! `GET /metrics` needs the service key. Series are keyed by the route *pattern* (for example
//! `/v1/projects/{project_id}/runs`), never the concrete path, so ids cannot blow up the label set.
use std::{collections::BTreeMap, sync::Mutex, time::Instant};

use axum::{extract::{MatchedPath, Request, State}, middleware::Next, response::Response};

use crate::state::AppState;

/// Upper bounds of the latency buckets, in seconds.
const BUCKETS: [f64; 10] = [0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 10.0];
/// Safety valve: past this many distinct series new ones are folded into one, so memory stays bounded.
const MAX_SERIES: usize = 500;

#[derive(Default)]
struct Series { count: u64, sum_seconds: f64, buckets: [u64; BUCKETS.len()] }

#[derive(Default)]
pub struct Metrics { series: Mutex<BTreeMap<(String, String, u16), Series>> }

impl Metrics {
    pub fn observe(&self, method: &str, route: &str, status: u16, seconds: f64) {
        let Ok(mut map) = self.series.lock() else { return };
        let mut key = (method.to_owned(), route.to_owned(), status);
        if !map.contains_key(&key) && map.len() >= MAX_SERIES { key = ("OTHER".into(), "other".into(), 0); }
        let s = map.entry(key).or_default();
        s.count += 1;
        s.sum_seconds += seconds;
        for (i, upper) in BUCKETS.iter().enumerate() { if seconds <= *upper { s.buckets[i] += 1; } }
    }

    pub fn render(&self) -> String {
        let mut out = String::from("# HELP helqix_http_requests_total Requests by method, route pattern and status.\n# TYPE helqix_http_requests_total counter\n");
        let Ok(map) = self.series.lock() else { return out };
        for ((m, r, st), s) in map.iter() { out += &format!("helqix_http_requests_total{{method=\"{m}\",route=\"{r}\",status=\"{st}\"}} {}\n", s.count); }
        out += "# HELP helqix_http_request_duration_seconds Request latency.\n# TYPE helqix_http_request_duration_seconds histogram\n";
        for ((m, r, st), s) in map.iter() {
            for (i, upper) in BUCKETS.iter().enumerate() { out += &format!("helqix_http_request_duration_seconds_bucket{{method=\"{m}\",route=\"{r}\",status=\"{st}\",le=\"{upper}\"}} {}\n", s.buckets[i]); }
            out += &format!("helqix_http_request_duration_seconds_bucket{{method=\"{m}\",route=\"{r}\",status=\"{st}\",le=\"+Inf\"}} {}\n", s.count);
            out += &format!("helqix_http_request_duration_seconds_sum{{method=\"{m}\",route=\"{r}\",status=\"{st}\"}} {}\n", s.sum_seconds);
            out += &format!("helqix_http_request_duration_seconds_count{{method=\"{m}\",route=\"{r}\",status=\"{st}\"}} {}\n", s.count);
        }
        out
    }
}

pub async fn record(State(state): State<AppState>, request: Request, next: Next) -> Response {
    let method = request.method().to_string();
    let route = request.extensions().get::<MatchedPath>().map_or_else(|| "unmatched".to_owned(), |p| p.as_str().to_owned());
    let start = Instant::now();
    let response = next.run(request).await;
    state.metrics.observe(&method, &route, response.status().as_u16(), start.elapsed().as_secs_f64());
    response
}

pub async fn expose(State(state): State<AppState>) -> ([(axum::http::HeaderName, &'static str); 1], String) {
    ([(axum::http::header::CONTENT_TYPE, "text/plain; version=0.0.4")], state.metrics.render())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_and_buckets_are_cumulative() {
        let m = Metrics::default();
        m.observe("GET", "/a", 200, 0.004);
        m.observe("GET", "/a", 200, 0.2);
        let t = m.render();
        assert!(t.contains("helqix_http_requests_total{method=\"GET\",route=\"/a\",status=\"200\"} 2"));
        assert!(t.contains("le=\"0.005\"} 1"), "only the fast request is under 5 ms:\n{t}");
        assert!(t.contains("le=\"0.25\"} 2") && t.contains("le=\"+Inf\"} 2"));
    }

    #[test]
    fn the_number_of_series_is_bounded() {
        let m = Metrics::default();
        for i in 0..(MAX_SERIES + 50) { m.observe("GET", &format!("/r{i}"), 200, 0.001); }
        assert!(m.render().contains("route=\"other\""));
        assert!(m.series.lock().unwrap().len() <= MAX_SERIES + 1);
    }
}
