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

## Bindings

This library provides C# bindings for easy integration with .NET applications.

See the [C# bindings contributing guide](./csharp/CONTRIBUTING.md) and [the C# bindings readme](./csharp/src/CliqueFusion/README.md) for details.

## 📜 Licensing

This project is publicly available under the **GNU General Public License v3.0**. It may optionally be distributed under the **MIT license by commercial arrangement.
