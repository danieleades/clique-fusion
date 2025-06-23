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
    #[must_use]
    pub fn cliques(&self) -> &[HashSet<Id>] {
        &self.cliques
    }

    /// Get the number of observations in the index
    #[must_use]
    pub fn len(&self) -> usize {
        self.compatibility_graph.len()
    }

    /// Check if the index is empty
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
