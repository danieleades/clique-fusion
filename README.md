# Clique Fusion

[![Latest Docs](https://docs.rs/clique-fusion/badge.svg)](https://docs.rs/clique-fusion/)
[![Continuous integration](https://github.com/danieleades/clique-fusion/actions/workflows/CI.yml/badge.svg)](https://github.com/danieleades/clique-fusion/actions/workflows/CI.yml)
[![codecov](https://codecov.io/gh/danieleades/clique-fusion/graph/badge.svg?token=4laxho1ik5)](https://codecov.io/gh/danieleades/clique-fusion)

**Clique Fusion** provides an efficient algorithm for grouping (or "fusing") multiple spatial observations of physical objects with uncertainty.

An *observation* is a discrete measurement of the position of an object (e.g., from sonar, camera, or manual marking), with an associated 2D Gaussian error model.

A single object may be observed multiple times — by different sensors, at different times, or from different passes. Due to measurement uncertainty, these observations may appear in different locations.

This library identifies and groups observations that are statistically consistent with originating from the same true object.

---

## 🔍 Fusion Logic

Two observations are considered compatible if their **difference is plausible** under the assumption that both are independent Gaussian samples from the same true position.

This is tested using the **Mahalanobis distance** between their positions under the **sum of their covariance matrices**. This approach is statistically optimal and robust to asymmetry or unequal sensor precision.

For example, if one observation has a coarse uncertainty and the other is precise, the fused compatibility test still produces a correct result.

---

## 🧠 Compatibility Test

For each observation, the algorithm searches for nearby candidates and checks whether they could represent the same object within a given confidence interval.

This is done by computing the squared Mahalanobis distance between their positions, using the combined uncertainty of both observations:

d² = (x₁ - x₂)ᵀ · (Σ₁ + Σ₂)⁻¹ · (x₁ - x₂)

If `d²` is less than a chi-squared threshold (e.g. 5.991 for 95% confidence in 2D), the observations are considered compatible.

---

## 🧮 Algorithm

The fusion process builds a compatibility graph and extracts cliques:

1. **Spatial Indexing**: Build an index to accelerate neighbour queries.
2. **Compatibility Filtering**: For each observation, query nearby candidates and test statistical compatibility using the formula above.
3. **Graph Construction**: Build an undirected graph linking all mutually compatible observations.
4. **Clique Detection**: Extract maximal cliques — each clique represents a group of mutually consistent observations that could correspond to a single real-world object.

---

## 🚀 Performance Notes

To avoid unnecessary comparisons, the library uses a spatial index and computes a maximum compatibility radius for each observation based on its own uncertainty and the maximum variance in the dataset.

> While the design supports future optimisation via **variance-aware ordering** (processing high-uncertainty observations first), this has not yet been implemented. Benchmarking will be used to determine whether the added complexity provides a meaningful performance benefit in practice.

---

## 📦 API

This library supports two usage modes:

- **Batch Mode**: Efficiently ingest a complete set of observations and compute all cliques in one pass.
- **Incremental Mode**: Insert observations one-by-one, maintaining compatibility graphs and clique structure on the fly — suitable for real-time or streaming applications.

---

## 📌 Use Cases

- Sensor fusion for autonomous mapping
- Multi-pass object detection and deduplication
- Observation deduplication in spatial databases

---

## 💡 Examples

### Creating Observations with Circular Error

Create an observation with a circular 95% confidence error region:

```rust
use clique_fusion::Observation;

// Observation at (10.5, 20.3) with a 5-meter circular 95% confidence error
let obs = Observation::builder(10.5, 20.3)
    .circular_95_confidence_error(5.0)
    .unwrap()
    .build();

assert_eq!(obs.x(), 10.5);
assert_eq!(obs.y(), 20.3);
```

### Constructing a Covariance Matrix

Create a covariance matrix representing positional uncertainty:

```rust
use clique_fusion::{CovarianceMatrix, Observation};

// 2x2 covariance matrix with:
// - variance in x direction: 4.0
// - variance in y direction: 2.0
// - covariance between x and y: 0.5
let cov = CovarianceMatrix::new(4.0, 2.0, 0.5).unwrap();

assert_eq!(cov.xx(), 4.0); // x variance
assert_eq!(cov.yy(), 2.0); // y variance
assert_eq!(cov.xy(), 0.5); // x-y covariance

// Create an observation with this custom covariance
let obs = Observation::builder(5.0, -3.0)
    .error(cov)
    .build();
```

### Testing Observation Compatibility

Check if two observations are statistically compatible (could originate from the same object):

```rust
use clique_fusion::{Observation, CHI2_2D_CONFIDENCE_95};

let obs1 = Observation::builder(0.0, 0.0)
    .circular_95_confidence_error(1.0)
    .unwrap()
    .build();

let obs2 = Observation::builder(1.5, 0.0)
    .circular_95_confidence_error(1.0)
    .unwrap()
    .build();

// Test compatibility at 95% confidence level
if obs1.is_compatible_with(&obs2, CHI2_2D_CONFIDENCE_95) {
    println!("Observations are likely from the same object");
} else {
    println!("Observations are likely from different objects");
}
```

### Batch Fusion

Fuse a batch of observations into groups (cliques) representing distinct objects:

```rust
use clique_fusion::{Observation, CliqueIndex, CHI2_2D_CONFIDENCE_95, Unique};

// Create observations from multiple sensors
let observations = vec![
    Unique {
        data: Observation::builder(0.0, 0.0)
            .circular_95_confidence_error(2.0)
            .unwrap()
            .build(),
        id: "sensor_1:obs_1"
    },
    Unique {
        data: Observation::builder(0.5, 0.3)
            .circular_95_confidence_error(2.0)
            .unwrap()
            .build(),
        id: "sensor_2:obs_1"
    },
    Unique {
        data: Observation::builder(50.0, 50.0)
            .circular_95_confidence_error(2.0)
            .unwrap()
            .build(),
        id: "sensor_3:obs_1"
    },
];

// Create clique index in batch mode (more efficient than incremental)
let index = CliqueIndex::from_observations(observations, CHI2_2D_CONFIDENCE_95);

// Retrieve cliques (groups of compatible observations)
let cliques = index.cliques();
println!("Found {} distinct objects", cliques.len());
```

### Incremental Fusion

Add observations one-by-one to maintain a live fusion state:

```rust
use clique_fusion::{Observation, CliqueIndex, CHI2_2D_CONFIDENCE_95, Unique};

// Start with an empty index
let mut index = CliqueIndex::new(CHI2_2D_CONFIDENCE_95);

// Add observations as they arrive
for (id, x, y) in &[
    ("obs_1", 10.0, 10.0),
    ("obs_2", 10.5, 10.3),
    ("obs_3", 100.0, 100.0),
] {
    let obs = Observation::builder(*x, *y)
        .circular_95_confidence_error(1.5)
        .unwrap()
        .build();

    index.insert(Unique { data: obs, id: id });
}

// Query results after each insertion
let cliques = index.cliques();
for (i, clique) in cliques.iter().enumerate() {
    println!("Object {}: {} observations", i, clique.len());
}
```

### Using Observation Context

Mark observations with shared context (e.g., from the same image or pass) to prevent false merging. Observations in the same context are assumed to have 0 covariance and hence are never merged into a clique:

```rust
use clique_fusion::{Observation, CovarianceMatrix};
use uuid::Uuid;

// All observations from the same image share a context
let image_id = Uuid::new_v4();
let cov = CovarianceMatrix::identity();

let obs1 = Observation::builder(5.0, 5.0)
    .error(cov)
    .context(image_id)
    .build();

let obs2 = Observation::builder(5.1, 5.1)
    .error(cov)
    .context(image_id)
    .build();

// Even though these observations are spatially very close,
// they remain separate because they share the same context
// (indicating they are distinct markings in the same image)
assert_eq!(obs1.context(), obs2.context());
```

---

## Bindings

This library provides C# bindings for easy integration with .NET applications.

See the [C# bindings contributing guide](./csharp/CONTRIBUTING.md) and [the C# bindings readme](./csharp/src/CliqueFusion/README.md) for details.

## 📜 Licensing

This project is publicly available under the **GNU General Public License v3.0**. It may optionally be distributed under the **MIT license by commercial arrangement.
