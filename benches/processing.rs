//! Bench tests for the Clique Fusion library.

#![allow(missing_docs)]

use std::num::NonZeroUsize;

use clique_fusion::{CHI2_2D_CONFIDENCE_95, CliqueIndex, Observation, Unique};
use criterion::{Criterion, criterion_group, criterion_main};

use uuid::Uuid;

mod gen_data;
use gen_data::{Config, generate_observations};

/// Read and parse JSONL file
fn five_pct_clustered() -> Vec<Unique<Observation, Uuid>> {
    let config = Config {
        spread: 500.0,
        cluster_size: 4.0,
        total_count: 5000,
        error_radius: 5.0,
        contacts_per_cluster: NonZeroUsize::new(4).unwrap(),
        random_seed: 12345,
        cluster_pct: 5.0,
    };
    generate_observations(&config)
}

fn benchmark_bulk(c: &mut Criterion) {
    let observations: Vec<_> = five_pct_clustered();

    c.bench_function("bulk_processing", |b| {
        b.iter(|| {
            let _index =
                CliqueIndex::from_observations(observations.clone(), CHI2_2D_CONFIDENCE_95);
        });
    });
}

fn benchmark_incremental(c: &mut Criterion) {
    let observations: Vec<_> = five_pct_clustered();

    c.bench_function("incremental_processing", |b| {
        b.iter(|| {
            let mut index = CliqueIndex::new(CHI2_2D_CONFIDENCE_95);
            for obs in &observations {
                index.insert(obs.clone());
            }
        });
    });
}

criterion_group!(benches, benchmark_bulk, benchmark_incremental);
criterion_main!(benches);
