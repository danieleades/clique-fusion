use std::collections::{HashMap, HashSet};

use crate::{Observation, Unique, cliques::find_maximal_cliques, spatial_index::SpatialIndex};

/// Groups mutually compatible observations into maximal cliques.
///
/// Each clique is a set of observation IDs whose error ellipses all overlap under a
/// chi-squared threshold. The index maintains a spatial index and compatibility graph
/// to make incremental updates efficient.
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
    /// Creates an empty index with a chi-squared threshold.
    ///
    /// # Example
    /// ```
    /// use clique_fusion::{CliqueIndex, CHI2_2D_CONFIDENCE_95};
    /// let index: CliqueIndex<i32> = CliqueIndex::new(CHI2_2D_CONFIDENCE_95);
    /// assert!(index.is_empty());
    /// ```
    #[must_use]
    pub fn new(chi2: f64) -> Self {
        Self {
            spatial_index: SpatialIndex::default(),
            compatibility_graph: HashMap::default(),
            cliques: Vec::default(),
            chi2,
        }
    }

    /// Builds an index from a batch of observations.
    ///
    /// Constructing the index in bulk is faster than inserting observations one by one.
    /// Observations sharing the same context are never fused into the same clique.
    ///
    /// # Example
    /// ```
    /// use clique_fusion::{Observation, Unique, CliqueIndex, CHI2_2D_CONFIDENCE_95};
    /// let obs = Observation::builder(0.0, 0.0)
    ///     .circular_95_confidence_error(5.0)
    ///     .unwrap()
    ///     .build();
    /// let index = CliqueIndex::from_observations(vec![Unique { id: 1, data: obs }], CHI2_2D_CONFIDENCE_95);
    /// assert_eq!(index.cliques().len(), 0);
    /// ```
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

    /// Inserts an observation and updates affected cliques.
    ///
    /// Observations that share a context are treated as distinct and never merged.
    ///
    /// # Example
    /// ```
    /// use clique_fusion::{Observation, Unique, CliqueIndex, CHI2_2D_CONFIDENCE_95};
    /// let mut index: CliqueIndex<i32> = CliqueIndex::new(CHI2_2D_CONFIDENCE_95);
    /// let obs = Observation::builder(0.0, 0.0)
    ///     .circular_95_confidence_error(5.0)
    ///     .unwrap()
    ///     .build();
    /// index.insert(Unique { id: 1, data: obs });
    /// assert!(index.cliques().is_empty());
    /// ```
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

            // Calculate affected region: new node + its direct neighbors (1-hop)
            // This is sufficient because:
            // - New node can only participate in cliques with its direct neighbors
            // - Only cliques containing the new node's neighbors can be affected
            // - Mutual compatibility ensures no "action at a distance" effects
            let mut affected = direct_neighbours;
            affected.insert(id); // New node is guaranteed to be in the graph at this point

            // Extract subgraph containing only affected nodes and their internal connections
            let subgraph = self.extract_subgraph(&affected).collect();

            // Recompute cliques in the affected subgraph
            let new_cliques = find_maximal_cliques(&subgraph);

            // Update global clique set: remove stale cliques and add new ones
            self.update_cliques(&affected, new_cliques);
        }
    }

    /// Extracts the portion of the compatibility graph spanned by `affected_nodes`.
    ///
    /// 1. Iterate over nodes in `affected_nodes`.
    /// 2. Look up each node's neighbours.
    /// 3. Keep only neighbours also in `affected_nodes`.
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

    /// Replaces cliques that overlap `affected_nodes` with `new_cliques`.
    fn update_cliques(&mut self, affected_nodes: &HashSet<Id>, new_cliques: Vec<HashSet<Id>>) {
        // Remove any existing cliques that overlap with the affected region
        // We need to remove these because they may no longer be maximal or may have merged
        self.cliques
            .retain(|clique| clique.is_disjoint(affected_nodes));

        // Add all newly computed cliques from the affected subgraph
        self.cliques.extend(new_cliques);
    }

    /// Returns all currently detected maximal cliques.
    #[must_use]
    pub fn cliques(&self) -> &[HashSet<Id>] {
        &self.cliques
    }

    /// Returns the number of observations in the index.
    #[must_use]
    pub fn len(&self) -> usize {
        self.compatibility_graph.len()
    }

    /// Returns `true` if the index has no observations.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.compatibility_graph.is_empty()
    }

    /// Exposes the compatibility graph for debugging and analysis.
    #[must_use]
    pub const fn compatibility_graph(&self) -> &HashMap<Id, HashSet<Id>> {
        &self.compatibility_graph
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use crate::{CHI2_2D_CONFIDENCE_95, CliqueIndex, Observation, Unique};

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
}
