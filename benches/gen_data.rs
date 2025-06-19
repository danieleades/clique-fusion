//! Generate reproducible test data for the bench tests.

use clique_fusion::{Observation, Unique};
use rand::prelude::*;
use std::{f64::consts::PI, num::NonZeroUsize};
use uuid::Uuid;

/// Configuration for generating synthetic observation data for benchmarking.
#[derive(Debug, Clone)]
pub struct Config {
    /// Maximum offset from reference point for scattered points, in meters.
    pub spread: f64,
    /// Percentage of points to generate in clusters (0–100).
    pub cluster_pct: f64,
    /// Maximum radius of each cluster in meters.
    pub cluster_size: f64,
    /// Number of observations per cluster.
    pub observations_per_cluster: NonZeroUsize,
    /// Total number of observations to generate.
    pub total_count: usize,
    /// The circular positional error of each observation's position in metres (95% confidence interval)
    pub error_radius: f64,
    /// Seed used by the random number generator
    pub random_seed: u64,
}

/// Generate a single point randomly distributed within a circle of a given radius
fn generate_scattered_point(radius: f64, rng: &mut impl Rng) -> (f64, f64) {
    fn limit_precision(value: f64) -> f64 {
        (value * 1e10).round() / 1e10
    }

    let distance = radius * rng.random::<f64>().sqrt();
    let angle = rng.random_range(0.0..2.0 * PI);

    let x = limit_precision(distance * angle.cos());
    let y = limit_precision(distance * angle.sin());
    (x, y)
}

/// Generates a random locations within a circular cluster.
fn generate_scatter(radius: f64, rng: &mut impl Rng) -> impl Iterator<Item = (f64, f64)> {
    std::iter::repeat_with(move || generate_scattered_point(radius, rng))
}

/// Iterator over clustered positions. Each cluster contains a fixed number of points, centred around a randomly chosen location.
///
/// This is a needlessly complicated approach, but it does mean that clustered positions can
/// be created entirely lazily, with no intermediate memory allocations.
pub struct ClusteredPositionIter<'a, R: Rng> {
    radius: f64,
    cluster_radius: f64,
    // This uses a non-zero integer since semantically a zero-sized cluster doesn't make sense.
    // This means we don't have to handle this case later in the iterator logic.
    cluster_count: NonZeroUsize,
    cluster_centre: (f64, f64),
    points_remaining: usize,
    rng: &'a mut R,
}

impl<'a, R> ClusteredPositionIter<'a, R>
where
    R: Rng,
{
    const fn new(
        radius: f64,
        cluster_radius: f64,
        cluster_count: NonZeroUsize,
        rng: &'a mut R,
    ) -> Self {
        Self {
            radius,
            cluster_radius,
            cluster_count,
            cluster_centre: (0.0, 0.0),
            points_remaining: 0,
            rng,
        }
    }
}

impl<R> Iterator for ClusteredPositionIter<'_, R>
where
    R: Rng,
{
    type Item = (f64, f64);

    fn next(&mut self) -> Option<Self::Item> {
        // If the cluster is exhausted, define a new cluster centre
        if self.points_remaining == 0 {
            self.points_remaining = self.cluster_count.into(); // always greater than 0!
            self.cluster_centre = generate_scattered_point(self.radius, self.rng);
        }

        // Generate a point within the cluster and return it
        let (dx, dy) = generate_scattered_point(self.cluster_radius, self.rng);
        self.points_remaining -= 1;
        Some((self.cluster_centre.0 + dx, self.cluster_centre.1 + dy))
    }
}

/// Generates synthetic observations in local (x, y) coordinates for benchmarking.
///
/// The output includes a mix of clustered and scattered observations.
#[allow(clippy::missing_panics_doc)]
#[must_use]
pub fn generate_observations(config: &Config) -> Vec<Unique<Observation, Uuid>> {
    let mut rng = StdRng::seed_from_u64(config.random_seed);

    // calculate distribution
    #[allow(
        clippy::cast_sign_loss,
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss
    )]
    let desired_clustered =
        ((config.total_count as f64) * (config.cluster_pct / 100.0)).round() as usize;
    #[allow(
        clippy::cast_sign_loss,
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss
    )]
    let num_clusters = (desired_clustered as f64
        / usize::from(config.observations_per_cluster) as f64)
        .ceil() as usize;
    let actual_clustered = num_clusters * usize::from(config.observations_per_cluster);

    // If we calculated more clustered than total, adjust
    let final_clustered = actual_clustered.min(config.total_count);
    let scattered_count = config.total_count - final_clustered;

    let mut observations = Vec::with_capacity(config.total_count);

    // Generate clustered observations
    for (x, y) in ClusteredPositionIter::new(
        config.spread,
        config.cluster_size,
        config.observations_per_cluster,
        &mut rng,
    )
    .take(final_clustered)
    {
        let observation = Observation::builder(x, y)
            .circular_95_confidence_error(config.error_radius)
            .unwrap()
            .build();
        observations.push(Unique {
            data: observation,
            id: Uuid::new_v4(),
        });
    }

    // Generate scattered observations
    for (x, y) in generate_scatter(config.spread, &mut rng).take(scattered_count) {
        let observation = Observation::builder(x, y)
            .circular_95_confidence_error(config.error_radius)
            .unwrap()
            .build();
        observations.push(Unique {
            data: observation,
            id: Uuid::new_v4(),
        });
    }

    // Verify we have the expected count
    assert_eq!(observations.len(), config.total_count);

    observations.shuffle(&mut rng);
    observations
}
