//! Utilities for generating pretty dotgraphs

use graphviz_rust::cmd::CommandArg;
use graphviz_rust::cmd::Format;
use graphviz_rust::dot_generator::*;
use graphviz_rust::dot_structures::*;
use graphviz_rust::exec;
use graphviz_rust::printer::DotPrinter;
use graphviz_rust::printer::PrinterContext;

use crate::control::control_nodes::ControlNode;
use crate::control::control_nodes::ControlNodeType;
use crate::control::CTreeNode;
use crate::control::CTreeNodeID;
use crate::control::ControlTree;
use crate::control::LeafNode;
use crate::control::RootNode;
use crate::control::ROOT_ID;
use crate::prelude::StandardDecorator;
use crate::traits::Decorator;
use crate::traits::ExecutorHook;
use crate::traits::UpdateCallback;
use crate::Status;

pub const SEQUENCE_SYMBOL: &str = "➡";
pub const FALLBACK_SYMBOL: &str = "?";
pub const PARALLEL_SYMBOL: &str = "⇉";
pub const LOOP_SYMBOL: &str = "↺";
pub const DECORATOR_SYMBOL: &str = "δ";
pub const INVERT_SYMBOL: &str = "!";
pub const SUBTREE_SYMBOL: &str = "🌳";

const INACTIVE_COLOR: &str = "gray";

pub trait GraphvizAttrs {
    fn graphviz_attrs(&self) -> Vec<Attribute>;
}

pub(crate) trait GraphvizNode {
    fn graphviz_node(&self) -> Node;
}

#[derive(Default)]
pub struct GraphvizAnimator {
    pub frames: Vec<Vec<u8>>,
}

impl GraphvizAnimator {
    pub fn save_html(&self, name: &str, frame_time: f32) {
        let html = self.render(frame_time);
        std::process::Command::new("mkdir")
            .args(["-p", "out"])
            .status()
            .unwrap();
        let path = format!("out/{name}.html");
        std::fs::write(&path, html).unwrap();
    }

    fn add_frame(&mut self, graph: Graph) {
        let mut ctx = PrinterContext::default();
        ctx.always_inline();
        let frame = exec(graph, &mut ctx, vec![CommandArg::Format(Format::Svg)]).unwrap();
        self.frames.push(frame);
    }

    /// Renders the frames as an html document.
    fn render(&self, frame_time: f32) -> String {
        let total_time = frame_time * self.frames.len() as f32;
        let (frames, classes): (Vec<_>, Vec<_>) = (0..self.frames.len())
            .map(|ix| {
                (
                    self.render_frame_html(ix),
                    Self::frame_css(ix, frame_time, total_time),
                )
            })
            .unzip();

        let mut buf = String::new();
        // css
        buf.push_str("<head>\n");
        buf.push_str("<style>\n");
        buf.push_str(
            "\
            body {\n\
                background-color: #222222;\n\
            }\n\
            polygon {\n\
                fill: #222222 !important;\n\
            }\n\
            text {\n\
                fill: white !important;\n\
            }\n\
            ",
        );
        buf.push_str(&self.keyframes_css());
        for class in classes {
            buf.push_str(&class);
        }
        buf.push_str("</style>\n");
        buf.push_str("</head>\n");

        // html
        buf.push_str("<body>\n");
        buf.push_str("<svg width=\"100%\" height=\"100%\">");
        for frame in frames {
            // graphviz outputs a <svg> for each frame, we don't want that, just the inner stuff
            let strip_svg = regex::Regex::new(r"<[/]?svg[^>]*>")
                .unwrap()
                .replace_all(&frame, "");
            buf.push_str(&strip_svg);
        }
        buf.push_str("</svg>\n");
        buf.push_str("</body>\n");

        buf
    }
    fn keyframes_css(&self) -> String {
        let keyframe_end = 100. / self.frames.len() as f32;
        format!(
            "\
            @keyframes show {{\n\
                0% {{ visibility: visible; }}\n\
                {keyframe_end}% {{ visibility: hidden; }}\n\
                100% {{ visibility: hidden; }}\n\
            }}
            "
        )
    }

    /// Get the css for a frame.
    fn frame_css(index: usize, frame_time: f32, total_time: f32) -> String {
        let id = Self::dom_id(index);
        let delay = frame_time * index as f32;
        let transition_time = frame_time * 0.5;
        format!(
            "\
            #{id} {{\n\
                visibility: hidden;\n\
                animation: {total_time}s show infinite;\n\
                animation-delay: {delay}s;\n\
                transition: visibility {transition_time}s {delay}s;\n\
            }}\n\
            "
        )
    }

    /// Get the id referenced by the css to do animation
    fn dom_id(index: usize) -> String {
        format!("frame{}", index)
    }

    /// Render the html for a frame.
    fn render_frame_html(&self, frame_index: usize) -> String {
        let frame_bytes = &self.frames[frame_index];
        let id = format!("id={}", Self::dom_id(frame_index));
        let frame_string = String::from_utf8(frame_bytes.to_vec()).unwrap();
        let id_removed = regex::Regex::new(r#"id="[^"]*""#)
            .unwrap()
            .replace_all(&frame_string, &id);

        let root_is_tick = id_removed.replace("Root", &format!("{frame_index}"));

        format!(
            "<g id={id}>\n\
                \t{root_is_tick}\n\
            </g>"
        )
    }
}

impl<D: Decorator + GraphvizAttrs> UpdateCallback<D> for GraphvizAnimator {
    fn callback(&mut self, state: &ControlTree<D>) {
        let graph = state.graphviz_graph();
        self.add_frame(graph);
    }
}

impl<D: Decorator + GraphvizAttrs> ControlTree<D> {
    /// Runs the control tree and saves the animation to `out/[name].html
    ///
    /// XXX: This writes a new svg for every frame, kinda scuffed & not good for performance so
    /// only use for debugging
    pub fn run_save_animation(
        &mut self,
        hook: &mut impl ExecutorHook,
        name: &str,
        frame_time: f32,
    ) -> Status {
        let animator = self.run_with_animatior(hook);
        animator.save_html(name, frame_time);
        self.status()
    }

    pub fn run_with_animatior<Hook: ExecutorHook>(&mut self, hook: &mut Hook) -> GraphvizAnimator {
        let mut animator = GraphvizAnimator::default();
        self.run_with_update_callback(hook, &mut animator);
        animator
    }

    /// Saves the control tree to `out/[name].dot`.
    pub fn save_dot(&self, name: &str) {
        let mut ctx = PrinterContext::default();
        let dot = self.graphviz_graph().print(&mut ctx);

        std::process::Command::new("mkdir")
            .args(["-p", "out"])
            .status()
            .unwrap();

        let path = format!("out/{name}.dot");

        std::fs::write(&path, dot).unwrap();
    }

    /// Get the [`graphviz_rust::Graph`] representation of the control tree in its current state.
    pub fn graphviz_graph(&self) -> Graph {
        let mut to_visit = vec![ROOT_ID];
        let mut stmts = vec![];

        while let Some(n) = to_visit.pop() {
            let parent = &self[n];
            let parent_node = parent.graphviz_node();
            let parent_id = parent_node.id.clone();
            stmts.push(stmt!(parent_node));

            let (child_nodes, edges_): (Vec<_>, Vec<_>) = self
                .iter_children(&n)
                .map(|child| {
                    to_visit.push(child.id().unwrap());
                    let child_node = child.graphviz_node();
                    let child_id = child_node.id.clone();
                    let parent_id = parent_id.clone();

                    let edge_attrs = if let Some(status) = child.status() {
                        match status {
                            Status::Failure => {
                                vec![
                                    attr!("arrowhead", "none"),
                                    attr!("arrowtail", "vee"),
                                    attr!("color", "red"),
                                    attr!("dir", "both"),
                                ]
                            }
                            Status::Running => {
                                vec![
                                    attr!("arrowhead", "vee"),
                                    attr!("style", "dashed"),
                                    attr!("color", "blue"),
                                ]
                            }
                            Status::Success => {
                                vec![
                                    attr!("arrowhead", "none"),
                                    attr!("arrowtail", "vee"),
                                    attr!("color", "green"),
                                    attr!("dir", "both"),
                                ]
                            }
                        }
                    } else {
                        let mut attrs = child.status().graphviz_attrs();
                        attrs.push(attr!("arrowhead", "empty"));
                        attrs
                    };

                    let edge = edge!(parent_id => child_id, edge_attrs);
                    (stmt!(child_node), stmt!(edge))
                })
                .unzip();

            stmts.extend(child_nodes);
            stmts.extend(edges_);
        }

        Graph::DiGraph {
            id: id!("ControlTree"),
            strict: true,
            stmts,
        }
    }
}

impl<D: Decorator + GraphvizAttrs> GraphvizNode for CTreeNode<D> {
    fn graphviz_node(&self) -> Node {
        match self {
            CTreeNode::Root(root) => root.graphviz_node(),
            CTreeNode::Leaf(leaf) => leaf.graphviz_node(),
            CTreeNode::Control(control) => control.graphviz_node(),
        }
    }
}

impl GraphvizNode for RootNode {
    fn graphviz_node(&self) -> Node {
        node!("root", self.graphviz_attrs())
    }
}

impl GraphvizAttrs for RootNode {
    fn graphviz_attrs(&self) -> Vec<Attribute> {
        let mut attrs = vec![];
        let label = format!("\"Root\\n{SEQUENCE_SYMBOL}\"");

        attrs.push(attr!("label", label));
        attrs.push(attr!("shape", "circle"));
        attrs.push(attr!("penwidth", "2.0"));

        let status_tip = format!("\"{}\"", status_str(self.0.status));
        attrs.push(attr!("tooltip", status_tip));

        attrs.extend(self.0.status.graphviz_attrs());
        attrs
    }
}

/* --- Status --- */
impl GraphvizAttrs for Option<Status> {
    fn graphviz_attrs(&self) -> Vec<Attribute> {
        if let Some(status) = self {
            status.graphviz_attrs()
        } else {
            vec![
                attr!("color", INACTIVE_COLOR),
                // attr!("tooltip", "\"Not run\""),
            ]
        }
    }
}

impl GraphvizAttrs for Status {
    fn graphviz_attrs(&self) -> Vec<Attribute> {
        let color = match self {
            Status::Success => "green",
            Status::Failure => "red",
            Status::Running => "blue",
        };
        vec![
            attr!("color", color),
            // attr!("tooltip", _tooltip)
        ]
    }
}

/* --- LeafNode --- */
impl GraphvizNode for LeafNode {
    fn graphviz_node(&self) -> Node {
        let id = format!("\"Leaf{}\"", self.id.unwrap().index());

        let label = self.name.clone().unwrap_or(id.clone());
        let mut attrs = self.graphviz_attrs();
        attrs.push(attr!("label", label));

        node!(id, attrs)
    }
}

impl GraphvizAttrs for LeafNode {
    fn graphviz_attrs(&self) -> Vec<Attribute> {
        let shape = match self.leaf_type {
            crate::control::LeafType::Conditional => "ellipse",
            crate::control::LeafType::Unknown | crate::control::LeafType::Executor => "box",
        };
        let mut attrs = vec![];
        attrs.push(attr!("shape", shape));

        let status_tip = format!("\"{}\"", status_str(self.status));
        let status_attrs = self.status.graphviz_attrs();

        attrs.push(attr!("tooltip", status_tip));
        attrs.extend(status_attrs);

        attrs
    }
}

fn status_str(status: Option<Status>) -> &'static str {
    match status {
        Some(Status::Success) => "Succeeded",
        Some(Status::Failure) => "Failed",
        Some(Status::Running) => "Running",
        None => "Never run",
    }
}

/* --- ControlNode --- */
impl<D: Decorator + GraphvizAttrs> GraphvizNode for ControlNode<D> {
    fn graphviz_node(&self) -> Node {
        let id = self.id.unwrap().graphviz_id();

        node!(id, self.graphviz_attrs())
    }
}

impl<D: Decorator + GraphvizAttrs> GraphvizAttrs for ControlNode<D> {
    fn graphviz_attrs(&self) -> Vec<Attribute> {
        let mut attrs = self.common_attrs();
        attrs.extend(self.node_type.graphviz_attrs());
        attrs
    }
}

impl<D: Decorator> ControlNode<D> {
    pub fn common_attrs(&self) -> Vec<Attribute> {
        let mut attrs = vec![];

        attrs.push(attr!("penwidth", "2.0"));

        let status_tip = status_str(self.status);

        // let shape = match &self.node_type {
        //     ControlNodeType::Decorator(_) => {
        //         attrs.push(attr!("width", "0.7"));
        //         attrs.push(attr!("height", "0.7"));
        //         "diamond"
        //     }
        //     _ => "square",
        // };
        let shape = "square";

        attrs.push(attr!("shape", shape));

        let tip = match &self.node_type {
            ControlNodeType::Sequence(_) => format!("\"Sequence ({status_tip})\""),
            ControlNodeType::Fallback(_) => format!("\"Fallback ({status_tip})\""),
            ControlNodeType::Parallel(_) => format!("\"Parallel ({status_tip})\""),
            ControlNodeType::Decorator(d) => {
                // let name = format!("\"{}\"", d.name());
                // attrs.push(attr!("xlabel", name));
                d.details()
                    .map(|deets| format!("\"{status_tip}: {deets}\""))
                    .unwrap_or(format!("\"Decorator ({status_tip})\""))
            }
        };
        attrs.push(attr!("tooltip", tip));

        attrs.extend(self.status.graphviz_attrs());
        attrs
    }
}

impl GraphvizAttrs for StandardDecorator {
    fn graphviz_attrs(&self) -> Vec<Attribute> {
        let symbol = match self {
            StandardDecorator::Invert(_) => INVERT_SYMBOL,
            StandardDecorator::Repeat(r) => &format!("{} \n {}", LOOP_SYMBOL, r.retry),
            StandardDecorator::Subtree(_) => SUBTREE_SYMBOL,
        };
        let symbol = format!("\"{symbol}\"");

        vec![attr!("label", symbol)]
    }
}

impl CTreeNodeID {
    pub fn graphviz_id(&self) -> Id {
        id!(format!("CTreeNodeId{}", self.index()))
    }
}

impl<D: Decorator + GraphvizAttrs> GraphvizAttrs for ControlNodeType<D> {
    fn graphviz_attrs(&self) -> Vec<Attribute> {
        let mut attrs = vec![];

        let symbol = match self {
            ControlNodeType::Sequence(_) => SEQUENCE_SYMBOL,
            ControlNodeType::Fallback(_) => FALLBACK_SYMBOL,
            ControlNodeType::Parallel(_) => PARALLEL_SYMBOL,
            ControlNodeType::Decorator(d) => return d.graphviz_attrs(),
        };
        let symbol = format!("\"{symbol}\"");
        attrs.push(attr!("label", symbol));

        attrs
    }
}
