use crate::vfs::VirtualFS;
use colored::Colorize;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;

/// Represents a node in the tree (either file or directory).
#[derive(Debug)]
struct TreeNode {
    name: String,
    children: Vec<Rc<RefCell<TreeNode>>>,
    is_file: bool,
}
impl TreeNode {
    fn new(name: String, is_file: bool) -> Self {
        Self {
            name,
            children: Vec::new(),
            is_file,
        }
    }
}

/// Build the directory tree from the VFS entries, returning the root node.
fn build_tree(vfs: &VirtualFS, destination: &Path) -> Rc<RefCell<TreeNode>> {
    // create a root node to represent the 'destination' directory
    let root_name = destination
        .file_name()
        .map(|os| os.to_string_lossy().to_string())
        .unwrap_or_else(|| destination.display().to_string());

    let root = Rc::new(RefCell::new(TreeNode::new(root_name, false)));

    // map full path to node
    let mut lookup: HashMap<String, Rc<RefCell<TreeNode>>> = HashMap::new();

    // insert the root folder in the map, keyed by its full string path.
    let root_str = destination.to_string_lossy().to_string();
    lookup.insert(root_str.clone(), Rc::clone(&root));

    // for each entry in your VFS, link it under its parent node.
    for entry in &vfs.entries {
        if let Some(rel_path) = &entry.destination {
            let full_path = destination.join(rel_path);
            let full_str = full_path.to_string_lossy().to_string();

            // identify the parent
            if let Some(parent_path) = full_path.parent() {
                let parent_str = parent_path.to_string_lossy().to_string();

                let parent_node = match lookup.get(&parent_str) {
                    Some(node) => Rc::clone(node),
                    None => {
                        // NOTE: If parent isn't yet in the map, maybe we could create the folders?
                        log::debug!(
                            "parent: {}, not found for path: {}",
                            parent_str,
                            rel_path.display()
                        );
                        continue;
                    }
                };

                // create a new node for file/directory
                let child_name = full_path
                    .file_name()
                    .map(|os| os.to_string_lossy().to_string())
                    .unwrap_or_else(|| full_str.clone());

                let new_child = Rc::new(RefCell::new(TreeNode::new(child_name, entry.is_file)));

                // push it under parent's children
                parent_node
                    .borrow_mut()
                    .children
                    .push(Rc::clone(&new_child));

                // insert into the map so future children can locate it
                lookup.insert(full_str, Rc::clone(&new_child));
            }
        }
    }

    root
}

/// Print the tree with a nice ASCII style.
fn print_tree(node: &Rc<RefCell<TreeNode>>, prefix: &str, is_last: bool) {
    let node_borrow = node.borrow();

    let connector = if is_last {
        "└── ".yellow()
    } else {
        "├── ".yellow()
    };
    let name = if node_borrow.is_file {
        node_borrow.name.green()
    } else {
        node_borrow.name.blue()
    };
    println!("{}{}{}", prefix.yellow(), connector, name);

    let child_prefix = if is_last {
        format!("{}    ", prefix.yellow())
    } else {
        format!("{}│   ", prefix.yellow())
    };

    let len = node_borrow.children.len();
    for (i, child) in node_borrow.children.iter().enumerate() {
        let last = i == len - 1;
        print_tree(child, &child_prefix, last);
    }
}

pub fn preview_as_tree(vfs: &VirtualFS, destination: &Path) {
    let tree_root = build_tree(vfs, destination);

    println!(
        "Legend: {} = (directory), {} = (file)",
        "blue".blue(),
        "green".green()
    );

    let fancy_prompt = format!(
        "{} {}\n",
        "┌─".bold().bright_blue(),
        "Preview".bold().bright_blue(),
    );

    println!("{}", fancy_prompt);

    print_tree(&tree_root, "", true);

    let fancy_prompt = format!(
        "\n\n{} {}\n",
        "└─".bold().bright_blue(),
        "Press [y] to confirm or [n] to cancel".bright_green()
    );

    println!("{}", fancy_prompt);
}
