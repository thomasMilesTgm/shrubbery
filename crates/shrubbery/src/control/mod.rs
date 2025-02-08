use crate::{traits::*, ShrubberyError, ShrubberyResult};
use ahash::HashMap;
use control_nodes::ControlNode;
use decorators::StandardDecorator;
use derive_more::From;

use crate::Status;

pub mod builder;
pub mod control_nodes;
pub mod decorators;
pub mod manipulation;
pub mod simple_executors;

pub const ROOT_ID: CTreeNodeID = CTreeNodeID(0);

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ChildUpdate {
    pub status: Status,
    pub child_id: CTreeNodeID,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CTreeNodeID(usize);

impl CTreeNodeID {
    pub fn index(&self) -> usize {
        self.0
    }
}

impl From<usize> for CTreeNodeID {
    fn from(id: usize) -> Self {
        Self(id)
    }
}

#[derive(Debug, Clone)]
pub struct ControlTree<D: Decorator> {
    pub(crate) nodes: Vec<CTreeNode<D>>,
    pub(crate) tree: HashMap<CTreeNodeID, Vec<CTreeNodeID>>,
}

pub type StdControlTree = ControlTree<StandardDecorator>;

impl<D: Decorator> Default for ControlTree<D> {
    fn default() -> Self {
        Self::new()
    }
}

impl<D: Decorator> std::ops::Index<CTreeNodeID> for ControlTree<D> {
    type Output = CTreeNode<D>;
    fn index(&self, index: CTreeNodeID) -> &Self::Output {
        &self.nodes[index.0]
    }
}

impl<D: Decorator> std::ops::IndexMut<CTreeNodeID> for ControlTree<D> {
    fn index_mut(&mut self, index: CTreeNodeID) -> &mut Self::Output {
        &mut self.nodes[index.0]
    }
}

impl<D: Decorator> ControlTree<D> {
    pub fn iter_control_nodes(&self) -> impl Iterator<Item = &ControlNode<D>> + '_ {
        self.nodes.iter().filter_map(|n| n.try_as_control())
    }
    pub fn iter_decorators(&self) -> impl Iterator<Item = &ControlNode<D>> + '_ {
        self.iter_control_nodes().filter(|c| c.is_decorator())
    }
    pub fn iter_tree(&self) -> impl Iterator<Item = (&CTreeNodeID, &Vec<CTreeNodeID>)> + '_ {
        self.tree.iter()
    }

    /// The status of the whole control tree (reflected by the status of the root node).
    pub fn status(&self) -> Status {
        self[ROOT_ID].status().unwrap_or_default()
    }

    pub fn run<Hook: ExecutorHook>(&mut self, hook: &mut Hook) -> Status {
        while self.status() == Status::Running {
            self.run_from(ROOT_ID, hook);
        }
        self.status()
    }

    pub fn run_with_update_callback<Hook: ExecutorHook, Callback: UpdateCallback<D>>(
        &mut self,
        hook: &mut Hook,
        cb: &mut Callback,
    ) -> Status {
        while self.status() == Status::Running {
            self.run_from_with_update_callback(ROOT_ID, hook, cb);
        }
        self.status()
    }

    pub fn run_from<Hook: ExecutorHook>(
        &mut self,
        node_id: CTreeNodeID,
        hook: &mut Hook,
    ) -> Status {
        self.run_from_with_update_callback(node_id, hook, &mut NoCallback)
    }

    pub fn run_from_with_update_callback<Hook: ExecutorHook, Callback: UpdateCallback<D>>(
        &mut self,
        node_id: CTreeNodeID,
        hook: &mut Hook,
        cb: &mut Callback,
    ) -> Status {
        let mut node_status = self[node_id].tick();
        cb.callback(self);

        while node_status.is_running() {
            for child in self.children(&node_id) {
                // tick the parent node & break if it's finished

                if self[node_id].tick().is_terminal() {
                    cb.callback(self);

                    break;
                }
                if self[child].status().unwrap_or_default().is_success() {
                    // don't re-run successful nodes
                    continue;
                }

                if let CTreeNode::Leaf(leaf) = &self[child] {
                    // hook the leaf node executor to get the status & update the control node with the
                    // result
                    let status = hook.hook(leaf);
                    self[child].set_status(status); // update the leaf node status from the hook

                    let update = ChildUpdate {
                        status,
                        child_id: child,
                    };
                    cb.callback(self);
                    self[node_id].child_updated(update);
                } else {
                    // continue down the control tree, updating the control node with the eventual
                    // result
                    let status = self[child].tick();
                    let subtree_status = match status {
                        Status::Running => self.run_from_with_update_callback(child, hook, cb),
                        _ => status,
                    };
                    let update = ChildUpdate {
                        status: subtree_status,
                        child_id: child,
                    };
                    self[node_id].child_updated(update);
                }
            }
            // tell the node all the children have run.
            self[node_id].all_children_seen();

            node_status = self[node_id].tick();
            self.handle_reset_requests(node_id);
            cb.callback(self);
        }
        node_status
    }

    fn handle_reset_requests(&mut self, node_id: CTreeNodeID) -> usize {
        if let Some(reset) = self[node_id]
            .try_as_control_mut()
            .map(|c| std::mem::take(&mut c.reset_requests))
        {
            reset
                .into_iter()
                .map(|id| {
                    self.reset_branch(id);
                })
                .count()
        } else {
            0
        }
    }

    pub fn reset_branch(&mut self, from: CTreeNodeID) {
        let mut to_visit = vec![from];
        while let Some(id) = to_visit.pop() {
            self[id].reset();

            self.tree[&id]
                .iter()
                .for_each(|&child| to_visit.push(child));
        }
    }

    pub fn new() -> Self {
        let root = CTreeNode::root();
        let mut tree = HashMap::<CTreeNodeID, Vec<CTreeNodeID>>::default();
        tree.insert(0.into(), vec![]);
        Self {
            nodes: vec![root],
            tree,
        }
    }

    /// Validate the tree for the following conditions, error if any are violated.
    ///
    /// - No cycles
    /// - No dangling control nodes
    /// - Decorators have only one child
    pub(crate) fn validate_bt_rules(&self) -> ShrubberyResult<()> {
        self.check_for_cycles()?;
        self.validate_decorators()?;
        self.check_for_dangling_control()?;
        Ok(())
    }

    /// Look for cycles in the tree, returns an error if any exist.
    pub(crate) fn check_for_cycles(&self) -> ShrubberyResult<()> {
        if let Some(err) = self.iter_tree().find_map(|(&parent, children)| {
            children.iter().find_map(|&child| {
                if let Err(e) = self.recurse_children_check_cycles(child, vec![parent]) {
                    Some(e)
                } else {
                    None
                }
            })
        }) {
            Err(err)
        } else {
            Ok(())
        }
    }

    /// Decorators are only allowed to have a single child
    pub(crate) fn validate_decorators(&self) -> ShrubberyResult<()> {
        if let Some(violation) = self
            .iter_decorators()
            .flat_map(|d| d.id)
            .find(|id| self.children(id).len() != 1)
        {
            Err(ShrubberyError::InvalidDecorator {
                decorator: violation,
                children: self.children(&violation),
            })
        } else {
            Ok(())
        }
    }

    /// Control nodes are by definition not leaf nodes so must have at least one child.
    pub(crate) fn check_for_dangling_control(&self) -> ShrubberyResult<()> {
        if let Some(dangling) = self
            .iter_control_nodes()
            .flat_map(|n| n.id)
            .find(|id| self.children(id).is_empty())
        {
            Err(ShrubberyError::DanglingControlNode(dangling))
        } else {
            Ok(())
        }
    }

    /// Recursively check for cycles in the tree, returning an error if any are found.
    fn recurse_children_check_cycles(
        &self,
        from: CTreeNodeID,
        mut history: Vec<CTreeNodeID>,
    ) -> ShrubberyResult<()> {
        if let Some(first) = history.first() {
            if first == &from {
                history.push(*first);
                return Err(ShrubberyError::CycleDetected(history));
            }
        }
        history.push(from);
        let children = self.children(&from);
        for child in children {
            self.recurse_children_check_cycles(child, history.clone())?;
        }
        Ok(())
    }

    /// Extract a section of the [`ControlTree`] into a new one.
    pub fn as_subtree(&self, start_at: CTreeNodeID) -> Self {
        let mut subtree = Self::new();

        let mut start = self[start_at].clone();
        let old_id = start.id().unwrap();

        start.unset_id();
        let new_id = subtree.add_child_unchecked(ROOT_ID, start);

        let mut old_to_new = HashMap::<CTreeNodeID, CTreeNodeID>::default();
        old_to_new.insert(ROOT_ID, ROOT_ID);
        old_to_new.insert(old_id, new_id);

        struct Deps<D: Decorator> {
            old_to_new: HashMap<CTreeNodeID, CTreeNodeID>,
            subtree: ControlTree<D>,
        }

        let mut deps = Deps {
            subtree,
            old_to_new,
        };

        self.explore_down_with_deps(start_at, &mut deps, |deps, parent, children| {
            let old_parent_id = &parent.id().unwrap();
            let parent_id = deps.old_to_new[old_parent_id];

            children.iter().for_each(|&old_id| {
                let new_id = deps.subtree.add_floating_node(self[old_id].clone());
                deps.subtree.tree.entry(parent_id).or_default().push(new_id);
                deps.old_to_new.insert(old_id, new_id);
            });
        });

        deps.subtree
    }

    fn add_floating_node(&mut self, node: impl Into<CTreeNode<D>>) -> CTreeNodeID {
        let node = node.into();
        let id = self.nodes.len().into();
        self.nodes.push(node);
        id
    }

    fn explore_down_with_deps<Deps>(
        &self,
        from: CTreeNodeID,
        deps: &mut Deps,
        f: impl Fn(&mut Deps, &CTreeNode<D>, &Vec<CTreeNodeID>),
    ) {
        let mut to_visit = vec![from];
        while let Some(from) = to_visit.pop() {
            let children = self.children(&from);
            let parent = &self[from];
            f(deps, parent, &children);
            for child in children {
                to_visit.push(child);
            }
        }
    }

    /// Insert `node` between `parent` and `move_down`.
    ///
    /// The inserted node with be the **n**'th child, where **n** is the index of the first child
    /// node in `move_down`. i.e. left->right order is maintained if `move_down` is contiguous with
    /// the original children.
    ///
    /// ## Examples
    ///
    /// ```rust,ignore
    /// insert_between(0, &[2], X)
    /// ```
    ///
    /// ```text
    ///         0                                  0
    ///       / | \            ------>           / | \
    ///      1  2  3                            1  X  3
    ///                                            |
    ///                                            2
    /// ```
    ///
    /// ```rust,ignore
    /// insert_between(0, &[1, 3], X)
    /// ```
    ///
    /// ```text
    ///         0                                 0
    ///       / | \            ------>           / \
    ///      1  2  3                            X   2
    ///                                        / \
    ///                                       1   3
    /// ```
    pub fn insert_between(
        &mut self,
        parent_id: CTreeNodeID,
        move_down: &[CTreeNodeID],
        node: impl Into<CTreeNode<D>>,
    ) -> CTreeNodeID {
        let node = node.into();
        let mut i = 0;
        self.tree
            .entry(parent_id)
            .and_modify(|children| {
                // find the index of the first child getting moved down -- this is where `node`
                // will be inserted.
                i = children
                    .iter()
                    .enumerate()
                    .find_map(|(i, c)| if move_down.contains(c) { Some(i) } else { None })
                    .expect("None of the children are in move_down");
                children.retain(|v| !move_down.contains(v))
            })
            .or_default();

        let new_id = self.add_child_unchecked(parent_id, node);

        self.tree.entry(parent_id).and_modify(|children| {
            children.pop();
            children.insert(i, new_id);
        });

        self.tree
            .entry(new_id)
            .or_default()
            .extend_from_slice(move_down);

        new_id
    }

    pub fn iter_children_mut<'a, O>(
        &'a mut self,
        node_id: &CTreeNodeID,
        mut f: impl FnMut(&mut CTreeNode<D>) -> O + 'a,
    ) -> impl Iterator<Item = O> + '_ {
        self.tree[node_id]
            .clone()
            .into_iter()
            .map(move |id| f(self.node_mut(id)))
    }

    pub fn node_mut(&mut self, id: CTreeNodeID) -> &mut CTreeNode<D> {
        &mut self.nodes[id.0]
    }

    pub fn children(&self, node_id: &CTreeNodeID) -> Vec<CTreeNodeID> {
        self.tree[node_id].clone()
    }

    pub fn iter_children(&self, node_id: &CTreeNodeID) -> impl Iterator<Item = &CTreeNode<D>> + '_ {
        self.tree[node_id].iter().map(|&id| &self[id])
    }

    pub fn iter_child_ids(&self, node_id: &CTreeNodeID) -> impl Iterator<Item = &CTreeNodeID> + '_ {
        self.tree[node_id].iter()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, From)]
pub enum CTreeNode<D: Decorator> {
    Root(RootNode),
    Control(ControlNode<D>),
    Leaf(LeafNode),
}

impl<D: Decorator> CTreeNode<D> {
    pub fn try_as_leaf_mut(&mut self) -> Option<&mut LeafNode> {
        match self {
            CTreeNode::Leaf(c) => Some(c),
            _ => None,
        }
    }
    pub fn try_as_leaf(&self) -> Option<&LeafNode> {
        match &self {
            CTreeNode::Leaf(c) => Some(c),
            _ => None,
        }
    }
    pub fn is_leaf(&self) -> bool {
        self.try_as_leaf().is_some()
    }
    pub fn try_as_control_mut(&mut self) -> Option<&mut ControlNode<D>> {
        match self {
            CTreeNode::Control(c) => Some(c),
            _ => None,
        }
    }

    pub fn try_as_control(&self) -> Option<&ControlNode<D>> {
        match &self {
            CTreeNode::Control(c) => Some(c),
            _ => None,
        }
    }
    pub fn is_control(&self) -> bool {
        self.try_as_control().is_some()
    }

    pub fn try_as_root(&self) -> Option<&RootNode> {
        match &self {
            CTreeNode::Root(c) => Some(c),
            _ => None,
        }
    }

    pub fn is_root(&self) -> bool {
        self.try_as_root().is_some()
    }

    pub fn root() -> Self {
        CTreeNode::Root(RootNode(ControlNode::sequence()))
    }
    pub fn leaf() -> Self {
        CTreeNode::Leaf(LeafNode::default())
    }
    pub fn unset_id(&mut self) {
        match self {
            CTreeNode::Root(root) => root.0.id = None,
            CTreeNode::Control(control) => control.id = None,
            CTreeNode::Leaf(leaf) => leaf.id = None,
        };
    }
    pub fn set_id(&mut self, id: CTreeNodeID) -> Option<CTreeNodeID> {
        let old = match self {
            CTreeNode::Root(root) => &mut root.0.id,
            CTreeNode::Control(control) => &mut control.id,
            CTreeNode::Leaf(leaf) => &mut leaf.id,
        };
        let mut id = Some(id);
        std::mem::swap(old, &mut id);
        id
    }
    pub fn id(&self) -> Option<CTreeNodeID> {
        match self {
            CTreeNode::Root(root) => root.0.id,
            CTreeNode::Control(control) => control.id,
            CTreeNode::Leaf(leaf) => leaf.id,
        }
    }
    pub fn reset(&mut self) {
        self.clear_status();
        match self {
            CTreeNode::Root(root) => root.0.reset(),
            CTreeNode::Control(control) => control.reset(),
            CTreeNode::Leaf(leaf) => leaf.reset(),
        }
    }
    pub fn clear_status(&mut self) {
        match self {
            CTreeNode::Root(root) => root.0.status = None,
            CTreeNode::Control(control) => control.status = None,
            CTreeNode::Leaf(leaf) => leaf.status = None,
        }
    }
    pub fn set_status(&mut self, status: Status) {
        match self {
            CTreeNode::Root(root) => root.0.status = Some(status),
            CTreeNode::Control(control) => control.status = Some(status),
            CTreeNode::Leaf(leaf) => leaf.status = Some(status),
        }
    }
    pub fn status(&self) -> Option<Status> {
        match self {
            CTreeNode::Root(root) => root.0.status,
            CTreeNode::Control(control) => control.status,
            CTreeNode::Leaf(leaf) => leaf.status,
        }
    }
}

impl<D: Decorator> Control for CTreeNode<D> {
    fn tick(&mut self) -> Status {
        let status = match self {
            CTreeNode::Root(root) => root.tick(),
            CTreeNode::Control(control) => control.tick(),
            CTreeNode::Leaf(leaf) => leaf.tick(),
        };
        self.set_status(status);
        status
    }
    fn child_updated(&mut self, update: ChildUpdate) {
        match self {
            CTreeNode::Root(root) => root.child_updated(update),
            CTreeNode::Control(control) => control.child_updated(update),
            CTreeNode::Leaf(leaf) => leaf.child_updated(update),
        }
    }

    fn all_children_seen(&mut self) {
        match self {
            CTreeNode::Root(r) => r.all_children_seen(),
            CTreeNode::Control(c) => c.all_children_seen(),
            CTreeNode::Leaf(l) => l.all_children_seen(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootNode(pub ControlNode<StandardDecorator>);

impl Control for RootNode {
    fn tick(&mut self) -> Status {
        self.0.tick()
    }
    fn child_updated(&mut self, update: ChildUpdate) {
        self.0.child_updated(update)
    }
    fn all_children_seen(&mut self) {
        self.0.all_children_seen()
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct LeafNode {
    pub id: Option<CTreeNodeID>,
    pub status: Option<Status>,
    pub details: Option<String>,
    pub name: Option<String>,
    pub leaf_type: LeafType,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub enum LeafType {
    #[default]
    Unknown,
    Conditional,
    Executor,
}

impl LeafNode {
    pub fn from_executor<BB: Blackboard, E: Executor<BB>>(executor: &E) -> Self {
        LeafNode {
            details: executor.details(),
            name: executor.name(),
            leaf_type: LeafType::Executor,
            ..Default::default()
        }
    }

    pub fn from_conditional<BB: Blackboard, C: Conditional<BB>>(conditional: &C) -> Self {
        LeafNode {
            details: conditional.details(),
            name: conditional.name(),
            leaf_type: LeafType::Conditional,
            ..Default::default()
        }
    }
    pub fn reset(&mut self) {
        self.status = None;
    }
}

impl Control for LeafNode {
    fn tick(&mut self) -> Status {
        self.status.unwrap_or_default()
    }
    fn child_updated(&mut self, _: ChildUpdate) {
        panic!("Leaf nodes should not have children");
    }
}
