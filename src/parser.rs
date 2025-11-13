use std::collections::{HashMap, HashSet};

use anyhow::{anyhow, Result};
use indexmap::IndexMap;
use once_cell::sync::Lazy;
use regex::Regex;

pub const DEFAULT_PADDING: i32 = 5;

#[derive(Clone, Debug)]
pub struct StyleClass {
    pub name: String,
    pub styles: HashMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TextNode {
    pub name: String,
    pub style_class: Option<String>,
}

#[derive(Clone, Debug)]
pub struct TextEdge {
    pub parent: TextNode,
    pub child: TextNode,
    pub label: String,
}

#[derive(Clone, Debug)]
pub struct TextSubgraph {
    pub name: String,
    pub nodes: Vec<String>,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GraphDirection {
    Lr,
    Td,
}

#[derive(Clone, Debug)]
pub struct GraphProperties {
    pub data: IndexMap<String, Vec<TextEdge>>,
    pub style_classes: HashMap<String, StyleClass>,
    pub graph_direction: GraphDirection,
    pub style_type: String,
    pub padding_x: i32,
    pub padding_y: i32,
    pub subgraphs: Vec<TextSubgraph>,
}

impl GraphProperties {
    fn add_node(&mut self, node: &TextNode) {
        self.data.entry(node.name.clone()).or_default();
    }

    fn set_data(&mut self, parent: &TextNode, edge: TextEdge) {
        self.data.entry(parent.name.clone()).or_default().push(edge);
        self.data.entry(edge.child.name.clone()).or_default();
    }

    fn set_arrow_with_label(
        &mut self,
        lhs: &[TextNode],
        rhs: &[TextNode],
        label: &str,
    ) -> Vec<TextNode> {
        for l in lhs {
            for r in rhs {
                let edge = TextEdge {
                    parent: l.clone(),
                    child: r.clone(),
                    label: label.to_string(),
                };
                self.set_data(l, edge);
            }
        }
        rhs.to_vec()
    }

    fn set_arrow(&mut self, lhs: &[TextNode], rhs: &[TextNode]) -> Vec<TextNode> {
        self.set_arrow_with_label(lhs, rhs, "")
    }

    fn parse_line(&mut self, line: &str) -> Result<Vec<TextNode>> {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return Ok(Vec::new());
        }
        for parser in PATTERNS.iter() {
            if let Some(capture) = parser.regex.captures(trimmed) {
                return (parser.handler)(self, capture);
            }
        }
        Err(anyhow!("Could not parse line: {}", line))
    }
}

struct Pattern {
    regex: &'static Regex,
    handler: fn(&mut GraphProperties, regex::Captures) -> Result<Vec<TextNode>>,
}

static EMPTY_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^\s*$").unwrap());
static ARROW_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^(.+)\s+-->\s+(.+)$").unwrap());
static ARROW_LABEL_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^(.+)\s+-->\|(.+)\|\s+(.+)$").unwrap());
static CLASS_DEF_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^classDef\s+(.+)\s+(.+)$").unwrap());
static AND_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^(.+)\s+&\s+(.+)$").unwrap());

static PATTERNS: Lazy<Vec<Pattern>> = Lazy::new(|| {
    vec![
        Pattern {
            regex: &EMPTY_REGEX,
            handler: |_, _| Ok(Vec::new()),
        },
        Pattern {
            regex: &ARROW_REGEX,
            handler: |gp, caps| {
                let lhs = gp.parse_line(caps.get(1).unwrap().as_str()).unwrap_or_else(|_| {
                    vec![parse_node(caps.get(1).unwrap().as_str())]
                });
                let rhs = gp.parse_line(caps.get(2).unwrap().as_str()).unwrap_or_else(|_| {
                    vec![parse_node(caps.get(2).unwrap().as_str())]
                });
                Ok(gp.set_arrow(&lhs, &rhs))
            },
        },
        Pattern {
            regex: &ARROW_LABEL_REGEX,
            handler: |gp, caps| {
                let lhs = gp.parse_line(caps.get(1).unwrap().as_str()).unwrap_or_else(|_| {
                    vec![parse_node(caps.get(1).unwrap().as_str())]
                });
                let rhs = gp.parse_line(caps.get(3).unwrap().as_str()).unwrap_or_else(|_| {
                    vec![parse_node(caps.get(3).unwrap().as_str())]
                });
                Ok(gp.set_arrow_with_label(
                    &lhs,
                    &rhs,
                    caps.get(2).unwrap().as_str(),
                ))
            },
        },
        Pattern {
            regex: &CLASS_DEF_REGEX,
            handler: |gp, caps| {
                let style = parse_style_class(
                    caps.get(1).unwrap().as_str(),
                    caps.get(2).unwrap().as_str(),
                );
                gp.style_classes.insert(style.name.clone(), style);
                Ok(Vec::new())
            },
        },
        Pattern {
            regex: &AND_REGEX,
            handler: |gp, caps| {
                let mut nodes = Vec::new();
                let left = gp.parse_line(caps.get(1).unwrap().as_str()).unwrap_or_else(|_| {
                    vec![parse_node(caps.get(1).unwrap().as_str())]
                });
                let right = gp.parse_line(caps.get(2).unwrap().as_str()).unwrap_or_else(|_| {
                    vec![parse_node(caps.get(2).unwrap().as_str())]
                });
                nodes.extend(left);
                nodes.extend(right);
                Ok(nodes)
            },
        },
    ]
});

fn parse_node(line: &str) -> TextNode {
    static NODE_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^(.+):::(.+)$").unwrap());
    if let Some(caps) = NODE_REGEX.captures(line.trim()) {
        TextNode {
            name: caps.get(1).unwrap().as_str().trim().to_string(),
            style_class: Some(caps.get(2).unwrap().as_str().trim().to_string()),
        }
    } else {
        TextNode {
            name: line.trim().to_string(),
            style_class: None,
        }
    }
}

fn parse_style_class(name: &str, styles: &str) -> StyleClass {
    let mut style_map = HashMap::new();
    for style in styles.split(',') {
        let mut parts = style.splitn(2, ':');
        if let (Some(key), Some(value)) = (parts.next(), parts.next()) {
            style_map.insert(key.trim().to_string(), value.trim().to_string());
        }
    }
    StyleClass {
        name: name.trim().to_string(),
        styles: style_map,
    }
}

pub fn mermaid_file_to_map(input: &str, style_type: &str) -> Result<GraphProperties> {
    let newline_pattern = Regex::new(r"\n|\\n").unwrap();
    let mut lines = Vec::new();
    for line in newline_pattern.split(input) {
        if line.trim() == "---" {
            break;
        }
        let trimmed = line.trim();
        if trimmed.starts_with("%%") {
            continue;
        }
        let mut processed = line.to_string();
        if let Some(idx) = processed.find("%%") {
            processed.truncate(idx);
        }
        let processed = processed.trim();
        if !processed.is_empty() {
            lines.push(processed.to_string());
        }
    }

    let mut properties = GraphProperties {
        data: IndexMap::new(),
        style_classes: HashMap::new(),
        graph_direction: GraphDirection::Lr,
        style_type: style_type.to_string(),
        padding_x: DEFAULT_PADDING,
        padding_y: DEFAULT_PADDING,
        subgraphs: Vec::new(),
    };

    let padding_regex = Regex::new(r"(?i)^padding([xy])\s*=\s*(\d+)$").unwrap();
    let mut idx = 0;
    while idx < lines.len() {
        let trimmed = lines[idx].trim();
        if trimmed.is_empty() {
            lines.remove(idx);
            continue;
        }
        if let Some(caps) = padding_regex.captures(trimmed) {
            let axis = caps.get(1).unwrap().as_str().to_ascii_lowercase();
            let value: i32 = caps.get(2).unwrap().as_str().parse()?;
            if axis == "x" {
                properties.padding_x = value;
            } else {
                properties.padding_y = value;
            }
            lines.remove(idx);
            continue;
        }
        break;
    }

    if lines.is_empty() {
        return Err(anyhow!("missing graph definition"));
    }

    match lines[0].trim() {
        "graph LR" | "flowchart LR" => properties.graph_direction = GraphDirection::Lr,
        "graph TD" | "flowchart TD" => properties.graph_direction = GraphDirection::Td,
        _ => return Err(anyhow!("first line should define the graph")),
    }

    let subgraph_regex = Regex::new(r"^\s*subgraph\s+(.+)$").unwrap();
    let end_regex = Regex::new(r"^\s*end\s*$").unwrap();
    let mut subgraph_stack: Vec<usize> = Vec::new();

    for line in lines.iter().skip(1) {
        let trimmed_line = line.trim();
        if let Some(caps) = subgraph_regex.captures(trimmed_line) {
            let name = caps.get(1).unwrap().as_str().trim().to_string();
            let parent = subgraph_stack.last().copied();
            let idx = properties.subgraphs.len();
            properties.subgraphs.push(TextSubgraph {
                name,
                nodes: Vec::new(),
                parent,
                children: Vec::new(),
            });
            if let Some(parent_idx) = parent {
                properties.subgraphs[parent_idx].children.push(idx);
            }
            subgraph_stack.push(idx);
            continue;
        }
        if end_regex.is_match(trimmed_line) {
            subgraph_stack.pop();
            continue;
        }

        let existing_nodes: HashSet<String> = properties.data.keys().cloned().collect();
        match properties.parse_line(line) {
            Ok(nodes) => {
                for node in nodes {
                    properties.add_node(&node);
                }
            }
            Err(_) => {
                let node = parse_node(line);
                properties.add_node(&node);
            }
        }

        if !subgraph_stack.is_empty() {
            for key in properties.data.keys() {
                if !existing_nodes.contains(key) {
                    for idx in &subgraph_stack {
                        let sg = properties
                            .subgraphs
                            .get_mut(*idx)
                            .expect("valid subgraph index");
                        if !sg.nodes.contains(key) {
                            sg.nodes.push(key.clone());
                        }
                    }
                }
            }
        }
    }

    Ok(properties)
}
