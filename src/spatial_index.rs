use std::collections::HashSet;

use rstar::{AABB, PointDistance, RTree, RTreeObject};

use crate::Observation;

/// A wrapper type that assigns a unique identifier to its payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unique<T, Id> {
    /// The wrapped payload.
    pub data: T,

    /// Unique identifier for the wrapped item.
    pub id: Id,
}

impl<Id> RTreeObject for Unique<Observation, Id> {
    type Envelope = AABB<[f64; 2]>;

    fn envelope(&self) -> Self::Envelope {
        AABB::from_point(self.data.position().into())
    }
}

impl<Id> PointDistance for Unique<Observation, Id> {
    fn distance_2(&self, point: &[f64; 2]) -> f64 {
        let (x, y) = self.data.position();
        let dx = x - point[0];
        let dy = y - point[1];
        dx.mul_add(dx, dy * dy)
    }
}

/// A spatial index supporting efficient nearest-neighbour and mutual-compatibility queries.
#[derive(Debug)]
pub struct SpatialIndex<Id> {
    tree: RTree<Unique<Observation, Id>>,

    /// The maximum variance of all observations in the index.
    ///
    /// This is used to determine the search radius needed to guarantee that all possible
    /// compatible neighbours have been considered when searching for neighbours.
    ///
    /// TODO: this could be optimised further by:
    ///
    /// - using a heap to track the variances in order
    /// - searching in descending order of variance
    /// - popping elements from the heap as they are searched
    /// - shrinking the search radius to match the updated maximum variance as you go
    ///
    /// benchmarking on large, representative datasets needed to determine whether this is worth it!
    max_variance: f64,
}

impl<Id> Default for SpatialIndex<Id> {
    fn default() -> Self {
        let tree = RTree::default();
        Self {
            tree,
            max_variance: 0.0,
        }
    }
}

impl<Id> SpatialIndex<Id>
where
    Id: PartialEq,
{
    /// Construct a spatial index from an initial list of observations.
    ///
    /// This is significantly faster than inserting observations individually via [`Self::insert`],
    /// especially for large numbers of items, due to bulk construction optimizations.
    ///
    /// See also: [`Self::insert`] for incremental use cases.
    #[must_use]
    pub fn from_observations(observations: Vec<Unique<Observation, Id>>) -> Self {
        let max_variance = observations
            .iter()
            .map(|obs| obs.data.error_covariance().max_variance())
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or(0.0);
        let tree = RTree::bulk_load(observations);
        Self { tree, max_variance }
    }

    /// Insert a single observation into the spatial index.
    ///
    /// Note: This method is slower than constructing the index in bulk via [`Self::from_observations`],
    /// and should be used only when dynamic insertion is required.
    ///
    /// See also: [`Self::from_observations`] for batch construction.
    ///
    /// # Panics
    ///
    /// Panics in debug builds if an observation with the same ID already exists in the index.
    pub fn insert(&mut self, observation: Unique<Observation, Id>) {
        debug_assert!(
            !self.tree.contains(&observation),
            "attempted to insert duplicate observation"
        );

        // Update the maximum variance
        self.max_variance = self
            .max_variance
            .max(observation.data.error_covariance().max_variance());

        self.tree.insert(observation);
    }
}

impl<Id> SpatialIndex<Id> {
    /// Find observations that are mutually compatible with a given query observation.
    ///
    /// Mutual compatibility means that both observations lie within each other's uncertainty
    /// ellipses under a specified chi-squared threshold. This is typically used to identify
    /// candidate pairs for sensor fusion.
    ///
    /// Observations that share the same *observation context* are excluded.
    /// This is important because the purpose of the algorithm is to identify pairs of observations
    /// that are consistent with originating from the *same* underlying object. However, if two
    /// observations are captured within the same context — for example, during the same sensor
    /// snapshot or measurement — then although absolute positioning error (e.g., platform GPS error)
    /// might be high, the *relative* error between those observations is negligible. In such cases,
    /// fusion is never appropriate, as we can perfectly distinguish them as separate entities.
    pub fn find_compatible<'a>(
        &'a self,
        query: &Unique<Observation, Id>,
        chi2_threshold: f64,
    ) -> impl Iterator<Item = &'a Unique<Observation, Id>>
    where
        Id: PartialEq,
    {
        let radius = query
            .data
            .max_compatibility_radius(chi2_threshold, self.max_variance);
        let p = query.data.position();

        self.tree
            .locate_within_distance(p.into(), radius)
            .filter(|other| query.id != other.id) // Exclude self
            .filter(|other| {
                // Skip observations from the same context (e.g. same measurement or snapshot).
                // If both observations have the same context, we assume they are distinct with negligible relative error,
                // and therefore should never be fused.
                !matches!((query.data.context(), other.data.context()), (Some(ctx1), Some(ctx2)) if ctx1 == ctx2)
            })
            .filter(move |obs| {
                obs.data
                    .is_compatible_with(&query.data, chi2_threshold)
            })
    }
}

impl<Id> SpatialIndex<Id>
where
    Id: PartialEq + Eq + std::hash::Hash + Copy,
{
    /// Build a graph connecting mutually compatible observations.
    ///
    /// The result is an undirected graph represented as an adjacency list, where each node is an
    /// observation ID and edges represent pairs of observations whose error ellipses mutually include
    /// the other's position under the given chi-squared threshold. Observations with no compatible
    /// neighbours are intentionally omitted to avoid emitting O(n) singleton cliques.
    pub fn compatibility_graph(
        &self,
        chi2_threshold: f64,
    ) -> impl Iterator<Item = (Id, HashSet<Id>)> {
        self.tree.iter().filter_map(move |obs| {
            let compatibles: HashSet<_> = self
                .find_compatible(obs, chi2_threshold)
                .map(|other| other.id)
                .collect();

            if compatibles.is_empty() {
                None
            } else {
                Some((obs.id, compatibles))
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::CovarianceMatrix;

    use super::*;

    #[test]
    fn find_compatible_excludes_self() {
        // Create a simple observation with circular error
        let obs_data = Observation::builder(0.0, 0.0)
            .circular_95_confidence_error(1.0)
            .unwrap()
            .build();
        let query_obs = Unique {
            data: obs_data,
            id: 1,
        };

        // Create an index with just this one observation
        let mut index = SpatialIndex::default();
        index.insert(query_obs.clone());

        // Find compatible observations
        let compatibles = index
            .find_compatible(&query_obs, crate::CHI2_2D_CONFIDENCE_95)
            .count();

        // Should be empty - the observation should not be compatible with itself
        assert_eq!(
            compatibles, 0,
            "find_compatible should not return the query observation itself"
        );
    }

    #[test]
    fn find_compatible_with_multiple_observations() {
        // Create multiple observations at the same location with different IDs
        let obs_data = Observation::builder(0.0, 0.0)
            .circular_95_confidence_error(2.0)
            .unwrap()
            .build();

        let obs1 = Unique {
            data: obs_data.clone(),
            id: 1,
        };
        let obs2 = Unique {
            data: obs_data.clone(),
            id: 2,
        };
        let obs3 = Unique {
            data: obs_data,
            id: 3,
        };

        let index = SpatialIndex::from_observations(vec![obs1.clone(), obs2.clone(), obs3.clone()]);

        // Find compatible observations for obs1
        let compatibles: Vec<_> = index
            .find_compatible(&obs1, crate::CHI2_2D_CONFIDENCE_95)
            .collect();

        // Should find obs2 and obs3, but not obs1 itself
        assert_eq!(
            compatibles.len(),
            2,
            "Should find 2 compatible observations"
        );
        assert!(
            !compatibles.iter().any(|obs| obs.id == obs1.id),
            "Should not include the query observation"
        );
        assert!(
            compatibles.iter().any(|obs| obs.id == obs2.id),
            "Should include obs2"
        );
        assert!(
            compatibles.iter().any(|obs| obs.id == obs3.id),
            "Should include obs3"
        );
    }

    #[test]
    fn find_compatible_with_overlapping_error_ellipses() {
        // Create observations that are close enough to be mutually compatible
        let cov_matrix = CovarianceMatrix::identity();

        let obs1 = Unique {
            data: Observation::builder(0.0, 0.0).error(cov_matrix).build(),
            id: 1,
        };
        let obs2 = Unique {
            data: Observation::builder(1.0, 0.0).error(cov_matrix).build(),
            id: 2,
        };
        let obs3 = Unique {
            data: Observation::builder(10.0, 0.).error(cov_matrix).build(), // Far away
            id: 3,
        };

        let index = SpatialIndex::from_observations(vec![obs1.clone(), obs2.clone(), obs3.clone()]);

        // Find compatible observations for obs1
        let compatibles: Vec<_> = index
            .find_compatible(&obs1, crate::CHI2_2D_CONFIDENCE_95)
            .collect();

        // Should find obs2 but not obs3 (too far) and not obs1 itself
        assert_eq!(compatibles.len(), 1, "Should find 1 compatible observation");
        assert!(
            !compatibles.iter().any(|obs| obs.id == obs1.id),
            "Should not include the query observation"
        );
        assert!(
            compatibles.iter().any(|obs| obs.id == obs2.id),
            "Should include obs2"
        );
        assert!(
            !compatibles.iter().any(|obs| obs.id == obs3.id),
            "Should not include obs3 (too far)"
        );
    }

    #[test]
    #[should_panic(expected = "attempted to insert duplicate observation")]
    fn disallows_duplicates() {
        let mut spatial_index = SpatialIndex::default();
        let observation = Unique {
            data: Observation::builder(0.0, 0.0)
                .circular_95_confidence_error(5.0)
                .unwrap()
                .build(),
            id: 0,
        };
        spatial_index.insert(observation.clone());
        spatial_index.insert(observation);
    }
}
