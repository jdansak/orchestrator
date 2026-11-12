//! Load generation and distribution assertions.
//!
//! The weighted-routing tests need to fire a known number of requests at a proxy and then check
//! that the traffic split matches the configured weights. The assertion uses the **binomial
//! distribution**, not an arbitrary tolerance, so a correct implementation does not fail
//! intermittently and a broken one does not pass by luck.

use std::collections::HashMap;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// The result of a load run.
#[derive(Debug, Clone)]
pub struct LoadResult {
    /// How many requests were sent.
    pub total: usize,
    /// How many requests succeeded.
    pub succeeded: usize,
    /// How many requests failed.
    pub failed: usize,
    /// The per-key counts, keyed by whatever the classifier returned.
    pub counts: HashMap<String, usize>,
}

/// Generates HTTP traffic against a proxy.
pub struct LoadGenerator {
    authority: String,
    path: String,
    requests: usize,
    concurrency: usize,
    classifier: Option<std::sync::Arc<dyn Fn(&[u8]) -> String + Send + Sync>>,
}

impl std::fmt::Debug for LoadGenerator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoadGenerator")
            .field("authority", &self.authority)
            .field("path", &self.path)
            .field("requests", &self.requests)
            .field("concurrency", &self.concurrency)
            .finish()
    }
}

impl LoadGenerator {
    /// Creates a generator for a proxy at the given authority.
    pub fn new(authority: impl Into<String>) -> Self {
        Self {
            authority: authority.into(),
            path: "/".to_string(),
            requests: 1_000,
            concurrency: 50,
            classifier: None,
        }
    }

    /// Sets the request path.
    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.path = path.into();
        self
    }

    /// Sets how many requests to send.
    pub fn requests(mut self, requests: usize) -> Self {
        self.requests = requests;
        self
    }

    /// Sets how many requests to send concurrently.
    pub fn concurrency(mut self, concurrency: usize) -> Self {
        self.concurrency = concurrency.max(1);
        self
    }

    /// Sets a classifier that reads the response body and returns a label.
    ///
    /// This is how a test learns which backend served a request.
    pub fn classify_by<F>(mut self, classifier: F) -> Self
    where
        F: Fn(&[u8]) -> String + Send + Sync + 'static,
    {
        self.classifier = Some(std::sync::Arc::new(classifier));
        self
    }

    /// Runs the load.
    pub async fn run(&self) -> LoadResult {
        let per_worker = self.requests.div_ceil(self.concurrency);
        let classifier = self.classifier.as_ref().map(std::sync::Arc::clone);
        let mut tasks = Vec::with_capacity(self.concurrency);

        for _ in 0..self.concurrency {
            let authority = self.authority.clone();
            let path = self.path.clone();
            let classifier = classifier.clone();

            tasks.push(tokio::spawn(async move {
                let mut counts: HashMap<String, usize> = HashMap::new();
                let mut succeeded = 0usize;
                let mut failed = 0usize;

                for _ in 0..per_worker {
                    match send_request(&authority, &path).await {
                        Ok(body) => {
                            succeeded += 1;
                            let label = match &classifier {
                                Some(classifier) => classifier(&body),
                                None => "default".to_string(),
                            };
                            *counts.entry(label).or_insert(0) += 1;
                        }
                        Err(_) => {
                            failed += 1;
                        }
                    }
                }

                (counts, succeeded, failed)
            }));
        }

        let mut counts: HashMap<String, usize> = HashMap::new();
        let mut succeeded = 0usize;
        let mut failed = 0usize;

        for task in tasks {
            let (task_counts, task_succeeded, task_failed) =
                task.await.expect("load task panicked");
            for (label, count) in task_counts {
                *counts.entry(label).or_insert(0) += count;
            }
            succeeded += task_succeeded;
            failed += task_failed;
        }

        LoadResult {
            total: succeeded + failed,
            succeeded,
            failed,
            counts,
        }
    }
}

/// Sends a single HTTP GET and returns the response body.
async fn send_request(authority: &str, path: &str) -> std::io::Result<Vec<u8>> {
    let mut stream = tokio::net::TcpStream::connect(authority).await?;
    let request = format!("GET {path} HTTP/1.1\r\nHost: {authority}\r\nConnection: close\r\n\r\n");
    stream.write_all(request.as_bytes()).await?;
    let mut response = Vec::new();
    stream.read_to_end(&mut response).await?;
    Ok(response)
}

/// Asserts that an observed count is within `tolerance` of an expected proportion.
///
/// This is the coarse check. [`assert_binomial`] is the statistically sound one.
pub fn assert_within(observed: usize, total: usize, expected: f64, tolerance: f64) {
    let actual = observed as f64 / total as f64;
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {expected:.4} (±{tolerance:.4}), observed {actual:.4} ({observed}/{total})"
    );
}

/// Asserts that an observed proportion is consistent with a binomial distribution.
///
/// Uses the exact binomial CDF rather than a normal approximation, so it stays correct for
/// extreme proportions and small sample counts.
pub fn assert_binomial(observed: usize, trials: usize, expected: f64, alpha: f64) {
    assert!(trials > 0, "the number of trials must be positive");

    let p = observed as f64 / trials as f64;
    let lower = binomial_quantile(trials, expected, alpha / 2.0);
    let upper = binomial_quantile(trials, expected, 1.0 - alpha / 2.0);

    assert!(
        p >= lower && p <= upper,
        "observed proportion {p:.4} is outside the {alpha:.4} confidence interval \
         [{lower:.4}, {upper:.4}] for an expected {expected:.4} over {trials} trials"
    );
}

/// The binomial quantile function, computed by direct summation.
///
/// Adequate for the sample sizes in the test suite. Not a general-purpose implementation.
pub fn binomial_quantile(n: usize, p: f64, q: f64) -> f64 {
    let mut cumulative = 0.0f64;
    let mut k = 0usize;

    while k <= n {
        cumulative += binomial_pmf(n, k, p);
        if cumulative >= q {
            return k as f64 / n as f64;
        }
        k += 1;
    }

    1.0
}

/// The probability mass function of the binomial distribution.
///
/// Computed in log space: the binomial coefficient overflows a `u128` well before 10,000
/// trials, which is exactly the sample size the routing tests use.
fn binomial_pmf(n: usize, k: usize, p: f64) -> f64 {
    if k > n {
        return 0.0;
    }
    if p <= 0.0 {
        return if k == 0 { 1.0 } else { 0.0 };
    }
    if p >= 1.0 {
        return if k == n { 1.0 } else { 0.0 };
    }

    let log_coefficient =
        ln_gamma(n as f64 + 1.0) - ln_gamma(k as f64 + 1.0) - ln_gamma((n - k) as f64 + 1.0);
    let log_probability = (k as f64) * p.ln() + ((n - k) as f64) * (1.0 - p).ln();

    (log_coefficient + log_probability).exp()
}

/// The natural logarithm of the gamma function, via the Lanczos approximation.
///
/// Accurate to about 15 significant digits for the arguments used here.
fn ln_gamma(x: f64) -> f64 {
    /// The Lanczos coefficients, for `g = 7` and `n = 9`.
    const COEFFICIENTS: [f64; 9] = [
        0.999_999_999_999_809_9,
        676.520_368_121_885_1,
        -1_259.139_216_722_402_8,
        771.323_428_777_653_1,
        -176.615_029_162_140_6,
        12.507_343_278_686_905,
        -0.138_571_095_265_720_12,
        9.984_369_578_019_572e-6,
        1.505_632_735_149_311_6e-7,
    ];

    if x < 0.5 {
        // The reflection formula.
        let sin = (std::f64::consts::PI * x).sin();
        if sin == 0.0 {
            return f64::INFINITY;
        }
        return std::f64::consts::PI.ln() - sin.ln().abs() - ln_gamma(1.0 - x);
    }

    let shifted = x - 1.0;
    let mut series = COEFFICIENTS[0];
    for (index, coefficient) in COEFFICIENTS.iter().enumerate().skip(1) {
        series += coefficient / (shifted + index as f64);
    }
    let t = shifted + 7.5;

    0.5 * (2.0 * std::f64::consts::PI).ln() + (shifted + 0.5) * t.ln() - t + series.ln()
}

/// Waits for a duration. Prefer a `FakeClock` in tests that are not measuring wall-clock
/// behaviour.
pub async fn settle(duration: Duration) {
    tokio::time::sleep(duration).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn within_tolerance_accepts_a_close_enough_result() {
        assert_within(600, 1_000, 0.6, 0.05);
    }

    #[test]
    #[should_panic(expected = "expected")]
    fn within_tolerance_rejects_a_far_result() {
        assert_within(700, 1_000, 0.6, 0.05);
    }

    #[test]
    fn the_binomial_interval_brackets_the_expectation() {
        let lower = binomial_quantile(10_000, 0.6, 0.025);
        let upper = binomial_quantile(10_000, 0.6, 0.975);
        assert!(
            lower < 0.6 && 0.6 < upper,
            "interval was [{lower}, {upper}]"
        );
    }

    #[test]
    fn the_binomial_check_accepts_a_correct_observation() {
        assert_binomial(6_000, 10_000, 0.6, 0.01);
    }

    #[test]
    #[should_panic(expected = "outside")]
    fn the_binomial_check_rejects_a_wrong_observation() {
        assert_binomial(9_000, 10_000, 0.6, 0.01);
    }

    #[test]
    fn ln_gamma_matches_known_values() {
        // ln(gamma(1)) = ln(1) = 0, ln(gamma(5)) = ln(24)
        assert!((ln_gamma(1.0) - 0.0).abs() < 1e-9);
        assert!((ln_gamma(5.0) - 24.0f64.ln()).abs() < 1e-9);
        // ln(gamma(0.5)) = ln(sqrt(pi))
        assert!((ln_gamma(0.5) - std::f64::consts::PI.sqrt().ln()).abs() < 1e-9);
    }

    #[tokio::test]
    async fn the_generator_counts_the_requests() {
        let result = LoadGenerator::new("127.0.0.1:1")
            .requests(10)
            .concurrency(2)
            .run()
            .await;

        // Port 1 is the discard port, so every request fails. What matters is that the
        // generator sends the right number and counts the failures.
        assert_eq!(result.total, 10);
        assert_eq!(result.failed, 10);
        assert_eq!(result.succeeded, 0);
    }

    #[tokio::test]
    async fn the_classifier_labels_the_responses() {
        let backend = crate::mock_backend::MockBackend::start().await;
        let result = LoadGenerator::new(backend.authority())
            .path("/api")
            .requests(5)
            .concurrency(1)
            .classify_by(|body| {
                if body.windows(6).any(|window| window == b"200 OK") {
                    "ok".to_string()
                } else {
                    "other".to_string()
                }
            })
            .run()
            .await;

        assert_eq!(result.succeeded, 5);
        assert_eq!(result.counts.get("ok"), Some(&5));
    }

    #[tokio::test]
    async fn debug_body() {
        let backend = crate::mock_backend::MockBackend::start().await;
        let result = LoadGenerator::new(backend.authority())
            .path("/api")
            .requests(1)
            .concurrency(1)
            .classify_by(|body| {
                format!(
                    "{:?}",
                    String::from_utf8_lossy(&body[..body.len().min(120)])
                )
            })
            .run()
            .await;
        println!("counts: {:?}", result.counts);
    }

    #[test]
    fn the_probability_mass_function_sums_to_one() {
        let mut total = 0.0;
        for k in 0..=100 {
            total += binomial_pmf(100, k, 0.6);
        }
        assert!((total - 1.0).abs() < 1e-9, "total was {total}");
    }
}
