#[derive(Debug, Eq, PartialEq)]
pub enum SortError<Node> {
    CycleDetected(Vec<(Node, Node)>),
}

impl<Node> std::error::Error for SortError<Node> where
    Node: Clone + Ord + core::fmt::Display + core::fmt::Debug
{
}

impl<Node: Clone + Ord + std::fmt::Display + std::fmt::Debug> std::fmt::Display
    for SortError<Node>
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            SortError::CycleDetected(edges) => {
                writeln!(f, "Cycle detected in the following DAG:")?;
                // Gather all unique nodes using Clone.
                let mut unique_nodes = std::collections::BTreeSet::new();
                for (src, dest) in edges.iter() {
                    unique_nodes.insert(src.clone());
                    unique_nodes.insert(dest.clone());
                }
                // Collect the unique nodes into a sorted vector.
                let sorted_nodes: Vec<Node> = unique_nodes.into_iter().collect();
                // Display the sorted nodes.
                writeln!(f, "Nodes:")?;
                for node in &sorted_nodes {
                    write!(f, "{} ", node)?;
                }
                writeln!(f, "\n")?;
                // Display edges with an arrow based on the order.
                writeln!(f, "Edges:")?;
                for (src, dest) in edges.iter() {
                    if src < dest {
                        writeln!(f, "  {} → {}", src, dest)?;
                    } else {
                        writeln!(f, "  {} ↖ {}", src, dest)?;
                    }
                }
                Ok(())
            }
        }
    }
}
