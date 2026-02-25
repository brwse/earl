use std::collections::HashMap;

/// Simplified AX node populated from CDP Accessibility.getFullAXTree.
#[derive(Debug, Clone)]
pub struct AXNode {
    pub backend_node_id: u64,
    pub role: String,
    pub name: String,
    pub children: Vec<AXNode>,
}

/// Render AX tree to markdown with opaque refs.
/// Returns (markdown_text, ref_id → backend_node_id map).
/// ref_id format: "e{backend_node_id}"
pub fn render_ax_tree(nodes: &[AXNode], max_nodes: usize) -> (String, HashMap<String, u64>) {
    let mut buf = String::new();
    let mut refs: HashMap<String, u64> = HashMap::new();
    let mut count = 0usize;
    render_nodes(nodes, 0, max_nodes, &mut count, &mut buf, &mut refs);
    if count >= max_nodes {
        buf.push_str(&format!(
            "\n[accessibility tree truncated at {max_nodes} nodes — increase max_snapshot_nodes if needed]"
        ));
    }
    (buf, refs)
}

fn render_nodes(
    nodes: &[AXNode],
    depth: usize,
    max: usize,
    count: &mut usize,
    buf: &mut String,
    refs: &mut HashMap<String, u64>,
) {
    let indent = "  ".repeat(depth);
    for node in nodes {
        if *count >= max {
            return;
        }
        let ref_id = format!("e{}", node.backend_node_id);
        refs.insert(ref_id.clone(), node.backend_node_id);
        buf.push_str(&format!(
            "{}- {} \"{}\" [ref={}]\n",
            indent, node.role, node.name, ref_id
        ));
        *count += 1;
        if !node.children.is_empty() {
            render_nodes(&node.children, depth + 1, max, count, buf, refs);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ax_tree_to_markdown_with_refs() {
        let nodes = vec![
            AXNode {
                backend_node_id: 1,
                role: "button".into(),
                name: "Login".into(),
                children: vec![],
            },
            AXNode {
                backend_node_id: 2,
                role: "textbox".into(),
                name: "Email".into(),
                children: vec![],
            },
        ];
        let (markdown, refs) = render_ax_tree(&nodes, 5000);
        assert!(
            markdown.contains("button \"Login\" [ref=e1]"),
            "got: {markdown}"
        );
        assert!(
            markdown.contains("textbox \"Email\" [ref=e2]"),
            "got: {markdown}"
        );
        assert_eq!(refs.get("e1"), Some(&1u64));
        assert_eq!(refs.get("e2"), Some(&2u64));
    }

    #[test]
    fn ax_tree_truncates_at_max_nodes() {
        let nodes: Vec<AXNode> = (1..=10)
            .map(|i| AXNode {
                backend_node_id: i,
                role: "button".into(),
                name: format!("btn{i}"),
                children: vec![],
            })
            .collect();
        let (markdown, refs) = render_ax_tree(&nodes, 5);
        assert!(
            markdown.contains("truncated"),
            "expected truncation notice, got: {markdown}"
        );
        assert_eq!(refs.len(), 5, "expected 5 refs, got {}", refs.len());
    }

    #[test]
    fn nested_children_rendered_with_indent() {
        let nodes = vec![AXNode {
            backend_node_id: 1,
            role: "list".into(),
            name: "nav".into(),
            children: vec![AXNode {
                backend_node_id: 2,
                role: "listitem".into(),
                name: "Home".into(),
                children: vec![],
            }],
        }];
        let (markdown, _) = render_ax_tree(&nodes, 5000);
        // Parent at depth 0, child at depth 1 (2-space indent)
        assert!(
            markdown.contains("  - listitem"),
            "expected indented child, got: {markdown}"
        );
    }

    #[test]
    fn empty_tree_returns_empty_string_and_empty_refs() {
        let (markdown, refs) = render_ax_tree(&[], 5000);
        assert!(
            markdown.is_empty() || !markdown.contains("[ref="),
            "got: {markdown}"
        );
        assert!(refs.is_empty());
    }
}
