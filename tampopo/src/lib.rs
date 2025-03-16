// ported and modified from: https://github.com/TheAlgorithms/Rust/blob/master/src/graph/topological_sort.rs
use errors::SortError;
use std::collections::{HashMap, VecDeque};
pub mod errors;

/// A type alias representing a directed graph as a list of edges, where each
pub type DAGAsAdjacencyList<Node> = Vec<(Node, Node)>;

/// A graph data structure used for topological sorting.
#[derive(Debug, Clone)]
pub struct Graph<Node> {
    /// Represents all nodes in the graph
    pub nodes: Vec<Node>,
    /// An adjacency list representing the directed edges between nodes.
    pub edges: DAGAsAdjacencyList<Node>,
}

/// An implementation of [Kahn's algorithm](https://en.wikipedia.org/wiki/Topological_sorting) for topological sorting.
///
/// Given a graph, this function returns a vector of nodes in a valid topological order.
/// If the graph contains a cycle, a `SortError::CycleDetected` error is returned.
/// # Example
/// ```
/// let nodes: Vec<usize> = vec![2, 3, 5, 7, 8, 9, 10, 11];
/// let edges: Vec<(usize, usize)> = vec![
///     (5, 11),
///     (7, 8),
///     (7, 11),
///     (3, 8),
///     (3, 10),
///     (11, 2),
///     (11, 9),
///     (11, 10),
///     (8, 9),
/// ];
/// let graph: Graph<usize> = Graph { nodes, edges };
/// let sorted = tampopo::sort_graph::<usize>(&graph);
///
/// assert!(sorted.is_ok());
/// ```
pub fn sort_graph<Node: std::hash::Hash + Eq + Clone>(
    graph: &Graph<Node>,
) -> Result<Vec<Node>, SortError<Node>> {
    // initialize data structures
    let mut dependencies_to_dependents_map: HashMap<Node, Vec<Node>> = HashMap::default();
    let mut in_degree_map: HashMap<Node, usize> = HashMap::default();
    // initialize the in-degree of all nodes to 0.
    for node in &graph.nodes {
        in_degree_map.entry(node.clone()).or_insert(0);
    }
    // build the dependency mapping and update in-degree counts based on graph edges.
    for (src, dest) in &graph.edges {
        dependencies_to_dependents_map
            .entry(src.clone())
            .or_default()
            .push(dest.clone());

        *in_degree_map.entry(dest.clone()).or_insert(0) += 1;
    }

    let mut queue: VecDeque<Node> = VecDeque::default();

    // add all nodes with zero in-degree to the queue.
    for (node, count) in &in_degree_map {
        if *count == 0 {
            queue.push_back(node.clone());
        }
    }

    let mut sorted: Vec<Node> = Vec::default();

    // process nodes from the queue, ensuring that dependencies are handled.
    while let Some(node_without_incoming_edges) = queue.pop_back() {
        sorted.push(node_without_incoming_edges.clone());

        in_degree_map.remove(&node_without_incoming_edges);

        // decrement the in-degree of each dependent node.
        for neighbor in dependencies_to_dependents_map
            .get(&node_without_incoming_edges)
            .unwrap_or(&vec![])
        {
            if let Some(count) = in_degree_map.get_mut(neighbor) {
                *count -= 1;

                // remove from in-degree map and add it to the queue if count becomes 0
                if *count == 0 {
                    in_degree_map.remove(neighbor);

                    queue.push_front(neighbor.clone());
                }
            }
        }
    }

    if in_degree_map.is_empty() {
        Ok(sorted)
    } else {
        Err(SortError::CycleDetected(graph.edges.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sort_graph_is_ok_integer() {
        let nodes: Vec<usize> = vec![2, 3, 5, 7, 8, 9, 10, 11];
        let edges: Vec<(usize, usize)> = vec![
            (5, 11),
            (7, 8),
            (7, 11),
            (3, 8),
            (3, 10),
            (11, 2),
            (11, 9),
            (11, 10),
            (8, 9),
        ];
        let graph: Graph<usize> = Graph { nodes, edges };
        let sorted = sort_graph::<usize>(&graph);

        assert!(sorted.is_ok());
    }

    #[test]
    fn test_sort_graph_is_err_integer() {
        let nodes: Vec<usize> = vec![2, 3, 5, 7, 8, 9, 10, 11];
        let edges: Vec<(usize, usize)> = vec![
            (5, 11),
            (7, 8),
            (7, 11),
            (3, 8),
            (3, 10),
            (11, 2),
            (11, 9),
            (11, 10),
            (8, 9),
            (9, 11), // <-- cycle introduced
        ];
        let graph: Graph<usize> = Graph { nodes, edges };
        let sorted = sort_graph::<usize>(&graph);

        assert!(sorted.is_err());
    }

    #[test]
    fn test_sort_graph_is_ok_strings() {
        let nodes = vec![
            "shirt",
            "hoodie",
            "socks",
            "underwear",
            "pants",
            "shoes",
            "glasses",
            "watch",
            "school",
        ];
        let edges = vec![
            ("shirt", "hoodie"),
            ("hoodie", "school"),
            ("underwear", "pants"),
            ("pants", "shoes"),
            ("socks", "shoes"),
            ("shoes", "school"),
        ];
        let graph: Graph<&str> = Graph { nodes, edges };
        let sorted = sort_graph::<&str>(&graph);

        assert!(sorted.is_ok());
    }

    #[test]
    fn test_is_err_strings() {
        let nodes = vec![
            "shirt",
            "hoodie",
            "socks",
            "underwear",
            "pants",
            "shoes",
            "glasses",
            "watch",
            "school",
        ];
        let edges = vec![
            ("shirt", "hoodie"),
            ("hoodie", "school"),
            ("school", "shirt"), // <-- cycle introduced
            ("underwear", "pants"),
            ("pants", "shoes"),
            ("socks", "shoes"),
            ("shoes", "school"),
        ];
        let graph: Graph<&str> = Graph { nodes, edges };
        let sorted = sort_graph::<&str>(&graph);

        assert!(sorted.is_err());
    }
}
