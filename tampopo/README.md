# Tampopo 🍜
Lightweight (💪 ) Rust library that implements topological sorting for directed acyclic graphs (DAGs) using [Kahn's algorithm](https://en.wikipedia.org/wiki/Topological_sorting#Kahn's_algorithm).

> Undiagnosed dyslexia might have made me type [tampopo](https://en.wikipedia.org/wiki/Tampopo) instead of topological. Not deep at all 🙃

![Human-made](https://img.shields.io/badge/Human--made-hotpink?style=flat-square&labelColor=ff69b4&color=ff1493)...mostly...🤭


## Example
```rs
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
```

### Acknowledgments
Ported and modified from [TheAlgorithms/Rust](https://github.com/TheAlgorithms/Rust/blob/master/src/graph/topological_sort.rs)
