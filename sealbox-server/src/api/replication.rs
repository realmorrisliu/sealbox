//! Watching whether the database is actually being replicated.
//!
//! Litestream replicates the database — not the master key, which is what the recovery blob is for
//! — and it does so silently. A replica that stopped three weeks ago looks exactly like a working
//! one until the day someone restores from it. That is the same failure as an unverified backup,
//! and the recovery ceremony already refuses to accept it for the master key; this refuses to
//! accept it for the database.
//!
//! Two rules, from Litestream's own metrics:
//!
//! - **`sync_count` must move.** Litestream syncs on a one-second ticker whether or not anything
//!   was written, so over a polling interval it has to advance. A count that stands still means
//!   Litestream is wedged, not that the server is quiet.
//! - **`sync_error_count` must not.** Any increase is a replica that is refusing writes —
//!   credentials expired, bucket gone, network partitioned.
//!
//! Nothing here fails `/healthz/ready`. A server that stops serving because its *backup* is stale
//! has turned a recoverable problem into an outage, and taking the machine out of rotation does
//! not fix replication. It is reported instead: loudly in the log, once per transition in the
//! audit trail, and on its own endpoint for anything that watches.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Serialize;
use tracing::{error, info};

/// How long to wait for the metrics endpoint. It is a loopback call to a process in the same
/// container; if it takes longer than this, something is wrong with it anyway.
const TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "snake_case", tag = "state")]
pub(crate) enum Health {
    /// No metrics URL configured. Not a failure — a local run replicates nowhere — but stated,
    /// so that nobody reads silence as "fine".
    NotConfigured,
    /// Not yet polled.
    Unknown,
    Healthy {
        syncs: u64,
    },
    /// Litestream is reachable and stuck, or unreachable, or erroring.
    Failing {
        reason: String,
    },
}

impl Health {
    fn label(&self) -> &'static str {
        match self {
            Health::NotConfigured => "not configured",
            Health::Unknown => "unknown",
            Health::Healthy { .. } => "healthy",
            Health::Failing { .. } => "failing",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct Status {
    pub health: Health,
    /// When this was last established, as a Unix timestamp. A stale timestamp with a healthy
    /// state means the watcher itself has stopped, which is worth being able to see.
    pub checked_at: Option<i64>,
}

#[derive(Clone)]
pub(crate) struct ReplicationWatch {
    status: Arc<Mutex<Status>>,
    last_counts: Arc<Mutex<Option<Counts>>>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Counts {
    syncs: u64,
    errors: u64,
}

impl ReplicationWatch {
    pub fn new(configured: bool) -> Self {
        Self {
            status: Arc::new(Mutex::new(Status {
                health: if configured {
                    Health::Unknown
                } else {
                    Health::NotConfigured
                },
                checked_at: None,
            })),
            last_counts: Arc::new(Mutex::new(None)),
        }
    }

    pub fn status(&self) -> Status {
        self.status.lock().unwrap().clone()
    }

    /// Poll once and return the health, along with whether it changed.
    ///
    /// The first poll only records a baseline: `sync_count` is cumulative, so movement cannot be
    /// judged until there are two samples.
    pub async fn poll(&self, url: &str) -> (Health, bool) {
        let health = match scrape(url).await {
            Err(reason) => Health::Failing { reason },
            Ok(counts) => {
                let previous = self.last_counts.lock().unwrap().replace(counts);
                match previous {
                    None => Health::Unknown,
                    Some(previous) if counts.errors > previous.errors => Health::Failing {
                        reason: format!(
                            "{} replication error(s) since the last check — the replica is \
                             refusing writes",
                            counts.errors - previous.errors
                        ),
                    },
                    Some(previous) if counts.syncs <= previous.syncs => Health::Failing {
                        reason: "Litestream has not synced since the last check. It syncs on a \
                                 timer whether or not anything was written, so a count that \
                                 stands still means it is wedged."
                            .to_string(),
                    },
                    Some(_) => Health::Healthy {
                        syncs: counts.syncs,
                    },
                }
            }
        };

        let mut status = self.status.lock().unwrap();
        let changed = status.health.label() != health.label();
        status.health = health.clone();
        status.checked_at = Some(time::OffsetDateTime::now_utc().unix_timestamp());
        (health, changed)
    }
}

/// Read the two counters out of Litestream's Prometheus output.
///
/// Summed across databases: sealbox has one, and a deployment that grew a second should have both
/// replicating rather than one masking the other.
async fn scrape(url: &str) -> std::result::Result<Counts, String> {
    let body = reqwest::Client::builder()
        .timeout(TIMEOUT)
        .build()
        .map_err(|e| format!("could not build a client: {e}"))?
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Litestream's metrics are unreachable at {url}: {e}"))?
        .error_for_status()
        .map_err(|e| format!("Litestream's metrics returned {e}"))?
        .text()
        .await
        .map_err(|e| format!("could not read Litestream's metrics: {e}"))?;

    let counts = Counts {
        syncs: sum_metric(&body, "litestream_sync_count"),
        errors: sum_metric(&body, "litestream_sync_error_count"),
    };

    // A metrics endpoint that answers but names no database is Litestream running with nothing to
    // replicate, which is exactly the silent nothing this exists to catch.
    if !body.contains("litestream_sync_count") {
        return Err("Litestream is serving metrics but replicating no database".to_string());
    }
    Ok(counts)
}

/// Prometheus text format: `name{labels} value`. Values are floats; these two are counters.
fn sum_metric(body: &str, name: &str) -> u64 {
    body.lines()
        .filter(|line| line.starts_with(name))
        .filter_map(|line| line.rsplit(' ').next())
        .filter_map(|value| value.parse::<f64>().ok())
        .map(|value| value as u64)
        .sum()
}

/// Poll forever, logging and auditing only when the answer changes.
///
/// Edge-triggered on purpose: a level-triggered version would write an audit record every minute
/// for as long as the problem lasted, and a trail nobody can read is a trail nobody reads.
pub(crate) async fn watch(state: crate::api::state::AppState, interval: Duration) {
    let Some(url) = state.config.replication_metrics_url.clone() else {
        return;
    };

    let mut ticker = tokio::time::interval(interval);
    loop {
        ticker.tick().await;
        let (health, changed) = state.replication.poll(&url).await;
        if !changed {
            continue;
        }

        let detail = match &health {
            Health::Failing { reason } => {
                error!("Replication is failing: {reason}");
                reason.clone()
            }
            Health::Healthy { syncs } => {
                info!("Replication is healthy again ({syncs} syncs)");
                "replication resumed".to_string()
            }
            other => other.label().to_string(),
        };

        let record = crate::repo::NewAuditRecord {
            identity: None,
            action: "replication".to_string(),
            resource: None,
            outcome: match health {
                Health::Healthy { .. } => crate::repo::AuditOutcome::Allowed,
                _ => crate::repo::AuditOutcome::Failed,
            },
            detail: Some(detail),
        };
        if let Err(e) = state.audit_repo.append(&record) {
            error!("Failed to record a replication change: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
litestream_db_size{db=\"/data/sealbox.db\"} 12288
litestream_sync_count{db=\"/data/sealbox.db\"} 42
litestream_sync_error_count{db=\"/data/sealbox.db\"} 0
litestream_wal_size{db=\"/data/sealbox.db\"} 16512
";

    #[test]
    fn counters_are_read_from_prometheus_text() {
        assert_eq!(sum_metric(SAMPLE, "litestream_sync_count"), 42);
        assert_eq!(sum_metric(SAMPLE, "litestream_sync_error_count"), 0);
        assert_eq!(sum_metric(SAMPLE, "litestream_nothing_like_this"), 0);
    }

    #[test]
    fn several_databases_are_summed_rather_than_one_masking_another() {
        let two = "\
litestream_sync_count{db=\"/data/a.db\"} 10
litestream_sync_count{db=\"/data/b.db\"} 5
";
        assert_eq!(sum_metric(two, "litestream_sync_count"), 15);
    }

    #[test]
    fn a_server_with_no_replication_says_so_rather_than_looking_healthy() {
        let watch = ReplicationWatch::new(false);
        assert_eq!(watch.status().health, Health::NotConfigured);
    }

    #[tokio::test]
    async fn an_unreachable_litestream_is_failing_not_unknown() {
        let watch = ReplicationWatch::new(true);
        // Port 1 is reserved and nothing listens there.
        let (health, changed) = watch.poll("http://127.0.0.1:1/metrics").await;
        assert!(matches!(health, Health::Failing { .. }), "{health:?}");
        assert!(changed, "unknown -> failing is a change worth recording");
    }
}
