use std::collections::{HashMap, HashSet};

use crate::{Observation, Unique, cliques::find_maximal_cliques, spatial_index::SpatialIndex};

/// An index which tracks the 'cliques' in the set of observations.
///
/// A 'clique' in this case represents a cluster of observations which lie mutually within each other's error ellipses,
/// and are therefore consistent with being observations of the same underlying object.
#[derive(Debug)]
pub struct CliqueIndex<Id> {
    spatial_index: SpatialIndex<Id>,
    compatibility_graph: HashMap<Id, HashSet<Id>>,
    cliques: Vec<HashSet<Id>>,
    chi2: f64,
}

impl<Id> CliqueIndex<Id>
where
    Id: Eq + std::hash::Hash + Copy + std::fmt::Debug,
{
    /// Construct a new index with a given confidence interval, defined by a Chi2 parameter
    #[must_use]
    pub fn new(chi2: f64) -> Self {
        Self {
            spatial_index: SpatialIndex::default(),
            compatibility_graph: HashMap::default(),
            cliques: Vec::default(),
            chi2,
        }
    }

    /// Construct a new index populated with an initial vector of observations.
    ///
    /// Constructing an index from a list of observations up front is much faster than adding them
    /// one at a time to an existing index.
    ///
    /// Note that observations in the same 'context' are never merged into cliques with each other, since
    /// they are assumed to have negligible relative error between them, and hence are distinguishable as
    /// separate objects.
    #[must_use]
    pub fn from_observations(observations: Vec<Unique<Observation, Id>>, chi2: f64) -> Self {
        let spatial_index = SpatialIndex::from_observations(observations);
        let compatibility_graph = spatial_index.compatibility_graph(chi2).collect();
        let cliques = find_maximal_cliques(&compatibility_graph);
        Self {
            spatial_index,
            compatibility_graph,
            cliques,
            chi2,
        }
    }

    /// Inserts a new observation, updating the spatial index, compatibility graph,
    /// and recomputing cliques in the affected subgraph.
    ///
    /// Note that observations in the same 'context' are never merged into cliques with each other, since
    /// they are assumed to have negligible relative error between them, and hence are distinguishable as
    /// separate objects.
    ///
    /// # Panics
    ///
    /// Panics on debug builds if an observation with the same ID already exists in the index.
    pub fn insert(&mut self, observation: Unique<Observation, Id>) {
        let id = observation.id;

        // 1. Identify mutually compatible neighbours
        let direct_neighbours: HashSet<Id> = self
            .spatial_index
            .find_compatible(&observation, self.chi2)
            .map(|obs| obs.id)
            .collect();

        // 2. Insert into spatial index
        self.spatial_index.insert(observation);

        // 3. Update compatibility graph and recompute cliques only if there are connections
        // If the new node has connections, update the compatibility graph and recompute cliques
        if !direct_neighbours.is_empty() {
            // Add the new node to the graph with its connections (sparse approach)
            self.compatibility_graph
                .insert(id, direct_neighbours.clone());

            // Add the new node to all its neighbors' adjacency lists
            for &neighbour in &direct_neighbours {
                self.compatibility_graph
                    .entry(neighbour)
                    .or_default()
                    .insert(id);
            }

            // Calculate affected region: closed neighbourhood of nodes whose adjacency changed
            // (the new node and all nodes we connected it to). Any clique that intersects this
            // set can change maximality because new edges may merge or supersede prior cliques.
            let mut affected = direct_neighbours;
            affected.insert(id); // New node is guaranteed to be in the graph at this point
            // Include existing neighbours of all changed nodes so we retain cliques such as
            // {A,B} where only A gained a new neighbour.
            for &node in &affected.clone() {
                if let Some(neighbours) = self.compatibility_graph.get(&node) {
                    affected.extend(neighbours.iter().copied());
                }
            }

            // Extract subgraph containing only affected nodes and their internal connections
            let subgraph = self.extract_subgraph(&affected).collect();

            // Recompute cliques in the affected subgraph
            let new_cliques = find_maximal_cliques(&subgraph);

            // Update global clique set: remove stale cliques and add new ones
            self.update_cliques(&affected, new_cliques);
        }
    }

    /// Extract subgraph containing only the specified nodes and edges between them
    ///
    /// The algorithm works as follows:
    /// 1. For each node in the affected region
    /// 2. Get all its neighbors from the full compatibility graph
    /// 3. Filter to only include neighbors that are also in the affected region
    /// 4. This creates a subgraph where only internal edges are preserved
    fn extract_subgraph(
        &self,
        affected_nodes: &HashSet<Id>,
    ) -> impl Iterator<Item = (Id, HashSet<Id>)> {
        affected_nodes.iter().map(|&node_id| {
            // Get all neighbors of this node from the full compatibility graph
            // This should always succeed since affected_nodes is built from graph traversal
            let all_neighbors = self
                .compatibility_graph
                .get(&node_id)
                .expect("Node in affected region must exist in compatibility graph");

            // Filter neighbors to only include those also in the affected region
            // This ensures we only preserve edges internal to the subgraph
            let subgraph_neighbors = all_neighbors
                .intersection(affected_nodes) // Set intersection: neighbors ∩ affected_nodes
                .copied()
                .collect();

            (node_id, subgraph_neighbors)
        })
    }

    /// Update the global clique set by removing stale cliques and adding new ones
    fn update_cliques(&mut self, affected_nodes: &HashSet<Id>, new_cliques: Vec<HashSet<Id>>) {
        // Remove any existing cliques that overlap with the affected region
        // We need to remove these because they may no longer be maximal or may have merged
        self.cliques
            .retain(|clique| clique.is_disjoint(affected_nodes));

        // Add all newly computed cliques from the affected subgraph
        self.cliques.extend(new_cliques);
    }

    /// Get the current set of maximal cliques
    ///
    /// Singleton cliques (isolated observations) are intentionally excluded to avoid
    /// generating O(n) trivial results. Only cliques containing two or more
    /// mutually compatible observations are returned.
    #[must_use]
    pub fn cliques(&self) -> &[HashSet<Id>] {
        &self.cliques
    }

    /// Get the number of observations in the index
    ///
    /// Note: this counts only observations that have at least one compatible neighbour
    /// (i.e. members of the compatibility graph). Isolated observations are tracked in
    /// the spatial index but deliberately omitted here for performance.
    #[must_use]
    pub fn len(&self) -> usize {
        self.compatibility_graph.len()
    }

    /// Check if the index is empty
    ///
    /// Returns true only when there are no mutually compatible pairs. If you insert
    /// observations that are all isolated, this will still return true.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.compatibility_graph.is_empty()
    }

    /// Get the compatibility graph (for debugging/analysis)
    #[must_use]
    pub const fn compatibility_graph(&self) -> &HashMap<Id, HashSet<Id>> {
        &self.compatibility_graph
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeSet, HashMap, HashSet};

    use crate::{CHI2_2D_CONFIDENCE_95, CliqueIndex, CovarianceMatrix, Observation, Unique};

    #[test]
    fn simple_cluster() {
        let observations = vec![
            Unique {
                data: Observation::builder(0.0, 0.0)
                    .circular_95_confidence_error(5.0)
                    .unwrap()
                    .build(),
                id: 0,
            },
            Unique {
                data: Observation::builder(0.0, 0.0)
                    .circular_95_confidence_error(5.0)
                    .unwrap()
                    .build(),
                id: 1,
            },
            Unique {
                data: Observation::builder(0.0, 0.0)
                    .circular_95_confidence_error(5.0)
                    .unwrap()
                    .build(),
                id: 2,
            },
        ];
        let index = CliqueIndex::from_observations(observations, CHI2_2D_CONFIDENCE_95);

        let expected = HashMap::from([
            (0, HashSet::from([1, 2])),
            (1, HashSet::from([0, 2])),
            (2, HashSet::from([0, 1])),
        ]);
        assert_eq!(index.compatibility_graph(), &expected);
    }

    #[test]
    fn no_overlap() {
        let observations = vec![
            Unique {
                data: Observation::builder(10.0, 0.0)
                    .circular_95_confidence_error(5.0)
                    .unwrap()
                    .build(),
                id: 0,
            },
            Unique {
                data: Observation::builder(0.0, 0.0)
                    .circular_95_confidence_error(5.0)
                    .unwrap()
                    .build(),
                id: 1,
            },
            Unique {
                data: Observation::builder(-10.0, 0.0)
                    .circular_95_confidence_error(5.0)
                    .unwrap()
                    .build(),
                id: 2,
            },
        ];
        let index = CliqueIndex::from_observations(observations, CHI2_2D_CONFIDENCE_95);

        let expected = HashMap::from([]);
        assert_eq!(index.compatibility_graph(), &expected);
    }

    #[test]
    fn insert_equivalence() {
        let observations = vec![
            Unique {
                data: Observation::builder(10.0, 0.0)
                    .circular_95_confidence_error(5.0)
                    .unwrap()
                    .build(),
                id: 0,
            },
            Unique {
                data: Observation::builder(0.0, 0.0)
                    .circular_95_confidence_error(5.0)
                    .unwrap()
                    .build(),
                id: 1,
            },
            Unique {
                data: Observation::builder(-10.0, 0.0)
                    .circular_95_confidence_error(5.0)
                    .unwrap()
                    .build(),
                id: 2,
            },
            Unique {
                data: Observation::builder(10.0, 0.0)
                    .circular_95_confidence_error(5.0)
                    .unwrap()
                    .build(),
                id: 3,
            },
            Unique {
                data: Observation::builder(10.0, 0.0)
                    .circular_95_confidence_error(5.0)
                    .unwrap()
                    .build(),
                id: 4,
            },
        ];

        let index1 = CliqueIndex::from_observations(observations.clone(), CHI2_2D_CONFIDENCE_95);

        let mut index2 = CliqueIndex::new(CHI2_2D_CONFIDENCE_95);

        for obs in observations {
            index2.insert(obs);
        }

        assert_eq!(index1.cliques, index2.cliques);
        assert_eq!(index1.compatibility_graph, index2.compatibility_graph);
    }

    #[test]
    fn incremental_insert_rebuilds_neighbour_cliques() {
        // Regression for the incremental-update bug: inserting a node connected to two existing
        // vertices must not drop cliques that involve their other neighbours.
        // Geometry yields edges AB, BC, AD, CD but not AC or BD (distance > 95% threshold)
        //          C (3,3)
        //          |
        // A (0,0)  B (0,3)   D (3,0)
        // Batch mode finds four edge cliques: {A,B}, {B,C}, {A,D}, {C,D}.
        // Previously the incremental path (A,B,C first, then D) recomputed only on {A,C,D},
        // so {A,B} and {B,C} were removed and never rebuilt. The fix recomputes the closed
        // neighbourhood of changed nodes, so the incremental result matches batch.
        let chi2 = CHI2_2D_CONFIDENCE_95;

        let obs_a = Unique {
            data: Observation::builder(0.0, 0.0)
                .error(CovarianceMatrix::identity())
                .build(),
            id: 0,
        };
        let obs_b = Unique {
            data: Observation::builder(0.0, 3.0)
                .error(CovarianceMatrix::identity())
                .build(),
            id: 1,
        };
        let obs_c = Unique {
            data: Observation::builder(3.0, 3.0)
                .error(CovarianceMatrix::identity())
                .build(),
            id: 2,
        };
        let obs_d = Unique {
            data: Observation::builder(3.0, 0.0)
                .error(CovarianceMatrix::identity())
                .build(),
            id: 3,
        };

        // Baseline: batch computation finds all 4 edge cliques in the 4-cycle graph
        let batch_index = CliqueIndex::from_observations(
            vec![obs_a.clone(), obs_b.clone(), obs_c.clone(), obs_d.clone()],
            chi2,
        );
        let expected = canonical_cliques(batch_index.cliques());

        // Incremental path: add A, B, C then insert D (connected to A and C only)
        let mut incremental = CliqueIndex::new(chi2);
        for obs in [&obs_a, &obs_b, &obs_c] {
            incremental.insert(obs.clone());
        }
        incremental.insert(obs_d);

        let incremental_cliques = canonical_cliques(incremental.cliques());

        assert_eq!(
            incremental_cliques, expected,
            "incremental maintenance should match batch computation, including neighbour cliques",
        );
    }

    fn canonical_cliques(cliques: &[HashSet<i32>]) -> Vec<BTreeSet<i32>> {
        let mut sorted: Vec<_> = cliques
            .iter()
            .map(|clique| clique.iter().copied().collect::<BTreeSet<_>>())
            .collect();
        sorted.sort();
        sorted
    }
}
