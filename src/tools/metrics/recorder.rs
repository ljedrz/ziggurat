//! Metrics recording types and utilities.

use std::collections::HashMap;

use histogram::Histogram;
use metrics::{Gauge, Key};
use metrics_util::{
    debugging::{DebugValue, DebuggingRecorder, Snapshotter},
    MetricKind,
};
use ordered_float::OrderedFloat;

/// A simple [`Snapshotter`](https://docs.rs/metrics-util/latest/metrics_util/debugging/struct.Snapshotter.html)
/// singleton implementation, that handles metrics for all registered counters, gauges and histograms.
///
/// Attempts to update unregistered metrics are ignored and logged to `std::err`. These metrics can
/// then be retrieved via the [`counters`], [`gauges`] and [`histograms`] getters. This recorder is
/// enabled by calling [`SimpleRecorder::default`].
pub struct SimpleRecorder(Snapshotter);

impl Default for SimpleRecorder {
    fn default() -> Self {
        // Let the errors through in case the recorder has already been set as is likely when running
        // multiple metrics tests.
        let recorder = DebuggingRecorder::new();
        let snapshotter = recorder.snapshotter();
        let _ = recorder.install();

        SimpleRecorder(snapshotter)
    }
}

impl SimpleRecorder {
    /// Map of all counters recorded.
    pub fn counters(&self) -> HashMap<Key, u64> {
        self.0
            .snapshot()
            .into_hashmap()
            .into_iter()
            .filter(|(key, _)| key.kind() == MetricKind::Counter)
            .map(|(key, (_, _, value))| {
                (
                    key.key().clone(),
                    if let DebugValue::Counter(c) = value {
                        c
                    } else {
                        unreachable!()
                    },
                )
            })
            .collect()
    }

    /// Map of all gauges recorded.
    pub fn gauges(&self) -> HashMap<Key, Gauge> {
        unreachable!("currently unused")
    }

    /// Map of all histograms recorded.
    pub fn histograms(&self) -> HashMap<Key, Histogram> {
        self.0
            .snapshot()
            .into_hashmap()
            .into_iter()
            .filter(|(key, _)| key.kind() == MetricKind::Histogram)
            .map(|(key, (_, _, value))| {
                (
                    key.key().clone(),
                    if let DebugValue::Histogram(h) = value {
                        create_histogram(h)
                    } else {
                        unreachable!()
                    },
                )
            })
            .collect()
    }

    /// Removes all previously registered metrics.
    pub fn clear(&self) {
        metrics::clear_recorder();
    }
}

impl Drop for SimpleRecorder {
    fn drop(&mut self) {
        // Clear the recorder to avoid the global state bleeding into other tests.
        metrics::clear_recorder();
    }
}

fn create_histogram(values: Vec<OrderedFloat<f64>>) -> Histogram {
    let mut histogram = Histogram::new();

    for v in values {
        let value = v.round() as u64;
        histogram.increment(value).unwrap();
    }

    histogram
}
