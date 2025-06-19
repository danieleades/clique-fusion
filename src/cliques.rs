use std::collections::{HashMap, HashSet};

/// Finds all maximal cliques in an undirected graph using the Bron-Kerbosch algorithm with pivoting.
///
/// A maximal clique is a complete subgraph (all vertices connected to each other) that cannot
/// be extended by adding another vertex. This implementation uses pivoting optimization to
/// reduce the search space significantly.
///
/// # Arguments
/// * `graph` - Adjacency list representation where each vertex maps to its neighbors
///
/// # Returns
/// Vector of all maximal cliques, where each clique is represented as a [`HashSet`] of vertex IDs
///
/// # Time Complexity
/// O(3^(n/3)) worst case, but typically much better with pivoting for sparse graphs
pub fn find_maximal_cliques<Id>(graph: &HashMap<Id, HashSet<Id>>) -> Vec<HashSet<Id>>
where
    Id: Copy + Eq + std::hash::Hash,
{
    if graph.is_empty() {
        return Vec::new();
    }

    // Pre-allocate with reasonable capacity - empirically, most graphs have O(n) cliques
    let mut cliques = Vec::with_capacity(graph.len().max(16));

    // Initialize Bron-Kerbosch sets
    let r = HashSet::new(); // Current clique (empty)
    let p = graph.keys().copied().collect(); // All vertices as candidates
    let x = HashSet::new(); // No excluded vertices initially

    bron_kerbosch_pivot(graph, r, p, x, &mut cliques);
    cliques
}

/// Optimized Bron-Kerbosch implementation with strategic pivoting.
///
/// This version includes several optimizations:
/// - Early termination checks
/// - Optimal pivot selection to minimize branching
/// - Efficient set operations using iterators where possible
/// - Memory-conscious cloning patterns
fn bron_kerbosch_pivot<Id>(
    graph: &HashMap<Id, HashSet<Id>>,
    r: HashSet<Id>,
    mut p: HashSet<Id>,
    mut x: HashSet<Id>,
    cliques: &mut Vec<HashSet<Id>>,
) where
    Id: Eq + std::hash::Hash + Copy,
{
    // Base case: found a maximal clique
    if p.is_empty() && x.is_empty() {
        cliques.push(r);
        return;
    }

    // Early termination: if P is empty but X is not, no maximal cliques possible
    if p.is_empty() {
        return;
    }

    // Select optimal pivot to minimize the number of recursive calls
    let candidates: Vec<_> = select_optimal_pivot(graph, &p, &x)
        .and_then(|pivot| graph.get(&pivot))
        // Process only vertices not connected to pivot (key optimization)
        // Convert to Vec to avoid iterator invalidation during P modification
        .map(|pivot_neighbors| p.difference(pivot_neighbors).copied().collect())
        .unwrap_or_default();

    for vertex in candidates {
        // Get vertex neighbors, defaulting to empty set for robustness
        let neighbors = graph.get(&vertex).cloned().unwrap_or_default();

        // Build next iteration state
        let mut r_next = r.clone();
        r_next.insert(vertex);

        let p_next = p.intersection(&neighbors).copied().collect();
        let x_next = x.intersection(&neighbors).copied().collect();

        // Recurse
        bron_kerbosch_pivot(graph, r_next, p_next, x_next, cliques);

        // Update P and X for next iteration (prevents duplicate cliques)
        p.remove(&vertex);
        x.insert(vertex);
    }
}

/// Selects the optimal pivot vertex to minimize recursive branching.
///
/// Strategy: Choose the vertex from P ∪ X with maximum degree in the induced subgraph.
/// This maximizes the number of vertices we can skip (those connected to the pivot).
///
/// # Performance Notes
/// - Uses iterator chains to avoid temporary allocations
/// - Caches the union computation for efficiency
/// - Handles empty sets gracefully
fn select_optimal_pivot<Id>(
    graph: &HashMap<Id, HashSet<Id>>,
    p: &HashSet<Id>,
    x: &HashSet<Id>,
) -> Option<Id>
where
    Id: Eq + std::hash::Hash + Copy,
{
    if p.is_empty() && x.is_empty() {
        return None;
    }

    // Find vertex with maximum neighbors in P ∪ X
    let px_union = p.iter().chain(x.iter());

    px_union
        .max_by_key(|&&vertex| {
            graph.get(&vertex).map_or(0, |neighbors| {
                // Count neighbors that are in P ∪ X
                neighbors
                    .iter()
                    .filter(|&n| p.contains(n) || x.contains(n))
                    .count()
            })
        })
        .copied()
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::*;

    /// Graph builder utility for constructing test graphs more ergonomically.
    ///
    /// This provides a fluent interface for building graphs in tests while maintaining
    /// the same internal representation used by the algorithm.
    struct GraphBuilder {
        vertices: Vec<Uuid>,
        graph: HashMap<Uuid, HashSet<Uuid>>,
    }

    impl GraphBuilder {
        fn with_vertices(count: usize) -> Self {
            let vertices: Vec<Uuid> = (0..count).map(|_| Uuid::new_v4()).collect();
            let mut graph = HashMap::with_capacity(count);

            // Initialize empty adjacency lists
            for &vertex in &vertices {
                graph.insert(vertex, HashSet::new());
            }

            Self { vertices, graph }
        }

        fn add_edge(mut self, u: usize, v: usize) -> Self {
            if u < self.vertices.len() && v < self.vertices.len() && u != v {
                let u_id = self.vertices[u];
                let v_id = self.vertices[v];

                self.graph.get_mut(&u_id).unwrap().insert(v_id);
                self.graph.get_mut(&v_id).unwrap().insert(u_id);
            }
            self
        }

        fn build(self) -> (HashMap<Uuid, HashSet<Uuid>>, Vec<Uuid>) {
            (self.graph, self.vertices)
        }
    }

    #[test]
    fn empty_graph_produces_no_cliques() {
        let cliques = find_maximal_cliques::<i32>(&HashMap::new());
        assert!(cliques.is_empty());
    }

    #[test]
    fn isolated_vertex_forms_singleton_clique() {
        let (graph, vertices) = GraphBuilder::with_vertices(1).build();

        let cliques = find_maximal_cliques(&graph);
        assert_eq!(cliques.len(), 1);
        assert_eq!(cliques[0].len(), 1);
        assert!(cliques[0].contains(&vertices[0]));
    }

    #[test]
    fn triangle_forms_single_3clique() {
        let (graph, vertices) = GraphBuilder::with_vertices(3)
            .add_edge(0, 1)
            .add_edge(1, 2)
            .add_edge(2, 0)
            .build();

        let cliques = find_maximal_cliques(&graph);
        assert_eq!(cliques.len(), 1);
        assert_eq!(cliques[0].len(), 3);

        // Verify all vertices are in the clique
        for &vertex in &vertices {
            assert!(cliques[0].contains(&vertex));
        }
    }

    #[test]
    fn path_graph_produces_edge_cliques() {
        // Path: 0-1-2-3 should produce cliques {0,1}, {1,2}, {2,3}
        let (graph, _) = GraphBuilder::with_vertices(4)
            .add_edge(0, 1)
            .add_edge(1, 2)
            .add_edge(2, 3)
            .build();

        let cliques = find_maximal_cliques(&graph);
        assert_eq!(cliques.len(), 3);

        // All cliques should be edges (size 2)
        for clique in &cliques {
            assert_eq!(clique.len(), 2);
        }
    }

    #[test]
    fn disconnected_components_produce_separate_cliques() {
        // Two edges: 0-1 and 2-3
        let (graph, _) = GraphBuilder::with_vertices(4)
            .add_edge(0, 1)
            .add_edge(2, 3)
            .build();

        let cliques = find_maximal_cliques(&graph);
        assert_eq!(cliques.len(), 2);

        for clique in &cliques {
            assert_eq!(clique.len(), 2);
        }
    }

    #[test]
    fn complete_graph_k4_has_single_4clique() {
        // Complete graph on 4 vertices - all connected to all
        let (graph, vertices) = GraphBuilder::with_vertices(4)
            .add_edge(0, 1)
            .add_edge(0, 2)
            .add_edge(0, 3)
            .add_edge(1, 2)
            .add_edge(1, 3)
            .add_edge(2, 3)
            .build();

        let cliques = find_maximal_cliques(&graph);
        assert_eq!(cliques.len(), 1);
        assert_eq!(cliques[0].len(), 4);

        // Should contain all vertices
        for &vertex in &vertices {
            assert!(cliques[0].contains(&vertex));
        }
    }

    #[test]
    fn handles_malformed_graph_gracefully() {
        // Graph with missing adjacency entries
        let mut graph = HashMap::new();

        // v1 references v2, but v2 has no entry in the graph
        graph.insert(1, std::iter::once(2).collect());
        // v2 is missing entirely

        let cliques = find_maximal_cliques(&graph);

        // Should handle gracefully without panicking
        assert!(!cliques.is_empty());
    }

    #[test]
    fn large_sparse_graph_performance() {
        // Create a graph with multiple disconnected triangles
        // Each triangle will form exactly one maximal clique of size 3
        let mut builder = GraphBuilder::with_vertices(999); // Use 99 to get exactly 33 triangles

        // Create disconnected triangles: (0,1,2), (3,4,5), (6,7,8), etc.
        for i in (0..999).step_by(3) {
            if i + 2 < 999 {
                builder = builder
                    .add_edge(i, i + 1)
                    .add_edge(i + 1, i + 2)
                    .add_edge(i + 2, i);
            }
        }

        let (graph, _) = builder.build();
        let cliques = find_maximal_cliques(&graph);

        // Should find exactly 333 triangular cliques (999/3)
        assert_eq!(cliques.len(), 333);

        // All cliques should be triangles
        for clique in &cliques {
            assert_eq!(clique.len(), 3);
        }
    }
}
