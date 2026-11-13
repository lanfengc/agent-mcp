use super::snapshot::{ScreenSize, UiElement, UiFlags};
use crate::bounds::Bounds;
use crate::error::{AppError, Result};
use roxmltree::{Document, Node};
use serde_json::json;

#[derive(Debug)]
pub(crate) struct ParsedUi {
    pub(crate) package: String,
    pub(crate) screen: ScreenSize,
    pub(crate) elements: Vec<UiElement>,
    pub(crate) node_count: usize,
}

struct RawNode {
    class: String,
    package: String,
    text: String,
    desc: String,
    resource_id: String,
    hint: String,
    bounds: Bounds,
    flags: UiFlags,
    children: Vec<RawNode>,
}

pub(crate) fn extract_xml(output: &str) -> Option<&str> {
    let start = output.find("<?xml")?;
    let tail = &output[start..];
    let end = tail
        .find("</hierarchy>")
        .map(|index| index + "</hierarchy>".len())
        .unwrap_or(tail.len());
    Some(&tail[..end])
}

pub(crate) fn parse_ui(xml: &str, max_depth: Option<u32>, all: bool) -> Result<ParsedUi> {
    let document = Document::parse(xml).map_err(|error| {
        AppError::new(
            "UI_PARSE_FAILED",
            format!("Could not parse the uiautomator hierarchy: {error}"),
        )
        .with_details(json!({ "xml_prefix": xml.chars().take(160).collect::<String>() }))
    })?;
    let root = document.root_element();
    let mut nodes = Vec::new();
    if root.tag_name().name() == "node" {
        if let Some(node) = parse_node(root, 0, max_depth) {
            nodes.push(node);
        }
    } else {
        for child in root
            .children()
            .filter(|child| child.is_element() && child.tag_name().name() == "node")
        {
            if let Some(node) = parse_node(child, 0, max_depth) {
                nodes.push(node);
            }
        }
    }
    let package = nodes
        .iter()
        .find(|node| !node.package.is_empty())
        .map(|node| node.package.clone())
        .unwrap_or_default();
    let screen = nodes
        .iter()
        .map(|node| node.bounds)
        .find(|bounds| !bounds.is_empty())
        .map(|bounds| ScreenSize {
            width: bounds.width() as u32,
            height: bounds.height() as u32,
        })
        .unwrap_or_default();
    let node_count = nodes.iter().map(count_nodes).sum();
    let mut elements = Vec::new();
    let mut counter = 0;
    flatten(nodes, all, &mut counter, &mut elements);
    Ok(ParsedUi {
        package,
        screen,
        elements,
        node_count,
    })
}

fn parse_node(node: Node<'_, '_>, depth: u32, max_depth: Option<u32>) -> Option<RawNode> {
    if max_depth.is_some_and(|limit| depth > limit) {
        return None;
    }
    let children = if max_depth.is_some_and(|limit| depth >= limit) {
        Vec::new()
    } else {
        node.children()
            .filter(|child| child.is_element() && child.tag_name().name() == "node")
            .filter_map(|child| parse_node(child, depth + 1, max_depth))
            .collect()
    };
    let class = attr(node, "class");
    Some(RawNode {
        package: attr(node, "package"),
        text: attr(node, "text"),
        desc: attr(node, "content-desc"),
        resource_id: attr(node, "resource-id"),
        hint: attr(node, "hint"),
        bounds: Bounds::parse(&attr(node, "bounds")).unwrap_or_default(),
        flags: UiFlags {
            clickable: bool_attr(node, "clickable"),
            long_clickable: bool_attr(node, "long-clickable"),
            scrollable: bool_attr(node, "scrollable"),
            checkable: bool_attr(node, "checkable"),
            checked: bool_attr(node, "checked"),
            enabled: node
                .attribute("enabled")
                .map(|value| value == "true")
                .unwrap_or(true),
            focused: bool_attr(node, "focused"),
            selected: bool_attr(node, "selected"),
            password: bool_attr(node, "password"),
        },
        class,
        children,
    })
}

fn attr(node: Node<'_, '_>, name: &str) -> String {
    node.attribute(name).unwrap_or("").to_string()
}

fn bool_attr(node: Node<'_, '_>, name: &str) -> bool {
    node.attribute(name) == Some("true")
}

fn count_nodes(node: &RawNode) -> usize {
    1 + node.children.iter().map(count_nodes).sum::<usize>()
}

fn flatten(nodes: Vec<RawNode>, all: bool, counter: &mut usize, elements: &mut Vec<UiElement>) {
    for mut node in nodes {
        let children = std::mem::take(&mut node.children);
        if all || node.is_interesting() {
            *counter += 1;
            let reference = format!("@a{counter}");
            elements.push(node.into_element(reference));
        }
        flatten(children, all, counter, elements);
    }
}

impl RawNode {
    fn is_interesting(&self) -> bool {
        self.flags.clickable
            || self.flags.long_clickable
            || self.flags.scrollable
            || self.flags.checkable
            || self.flags.password
            || self.class.ends_with("EditText")
            || !self.text.is_empty()
            || !self.desc.is_empty()
            || !self.resource_id.is_empty()
            || !self.hint.is_empty()
    }

    fn into_element(self, reference: String) -> UiElement {
        let bounds = self.bounds;
        let role = short_role(&self.class);
        UiElement {
            reference,
            role,
            class: self.class,
            text: non_empty(self.text),
            desc: non_empty(self.desc),
            resource_id: non_empty(self.resource_id),
            hint: non_empty(self.hint),
            package: non_empty(self.package),
            bounds,
            center: bounds.center(),
            flags: self.flags,
        }
    }
}

fn short_role(class: &str) -> String {
    if class.is_empty() {
        return "View".to_string();
    }
    class
        .rsplit('.')
        .next()
        .filter(|value| !value.is_empty())
        .unwrap_or("View")
        .to_string()
}

fn non_empty(value: String) -> Option<String> {
    if value.is_empty() { None } else { Some(value) }
}
