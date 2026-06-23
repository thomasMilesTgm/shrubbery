use std::fmt::Debug;

use crate::control::{CTreeNodeID, ChildUpdate, ControlTree, LeafNode};
use crate::Status;

pub trait Control {
    /// Tick the control node, returning the status of the node
    fn tick(&mut self) -> Status;

    /// Register the returned status from a child node
    fn child_updated(&mut self, update: ChildUpdate);

    /// Register the fact you have now seen all the children.
    ///
    /// Default implementation does nothing, but
    /// [`Sequence`](crate::control::control_nodes::Sequence) and others need to know when to
    /// return success, otherwise they will be stuck [`Status::Running`] forever.
    fn all_children_seen(&mut self) {}
}

/// Connector types that define what to do when the [`ControlTree`] ticks a leaf node.
pub trait ExecutorHook {
    fn hook(&mut self, leaf: &LeafNode) -> Status;
}

pub trait Decorator: Clone {
    /// Initialize the decorator
    fn init(&mut self);

    /// Apply the decorator to a [`ChildUpdate`]
    fn child_updated(&mut self, update: ChildUpdate) -> Status;

    /// What is the current status of this node?
    fn status(&self) -> Status;

    fn reset(&mut self);

    fn name(&self) -> String;

    fn details(&self) -> Option<String> {
        None
    }

    /// Request a group of nodes get reset
    fn reset_request(&mut self) -> Option<CTreeNodeID> {
        None
    }
}

/// Callback that can be used during the exploration of the [`ControlTree`]. Useful primarily for
/// debuggers such as the [`GraphvizAnimator`](crate::graphviz::GraphvizAnimator), for diagnosing
/// the behavior inside the control tree itself, regardless of what the leaf nodes & blackboard are
/// doing internally.
pub trait UpdateCallback<D: Decorator> {
    /// Called when there are noteworthy events in [`ControlTree::run_from_with_update_callback`]
    fn callback(&mut self, state: &ControlTree<D>);
}

/// No-op callback
pub struct NoCallback;

impl<D: Decorator> UpdateCallback<D> for NoCallback {
    fn callback(&mut self, _state: &ControlTree<D>) {}
}

/// Leaf nodes that execute a task & update the state of the [`Blackboard`].
pub trait Executor<BB: Blackboard>: Clone + Debug {
    fn execute(&self, blackboard: &mut BB) -> Status;

    /// Optional name for coloring the leaf nodes in the [`ControlTree`]
    fn name(&self) -> Option<String> {
        None
    }

    /// Optional details for coloring the leaf nodes in the [`ControlTree`]
    fn details(&self) -> Option<String> {
        None
    }
}

/// Leaf nodes that read the [`Blackboard`] and return a [`Status`] about it.
pub trait Conditional<BB: Blackboard>: Clone + Debug {
    fn conditional(&self, blackboard: &BB) -> Status;

    /// Optional name for coloring the leaf nodes in the [`ControlTree`]
    fn name(&self) -> Option<String> {
        None
    }

    /// Optional details for coloring the leaf nodes in the [`ControlTree`]
    fn details(&self) -> Option<String> {
        None
    }
}

/// The blackboard is a shared state of the behavior tree that is updated by [`Executor`] leaf
/// nodes, and read by [`Conditional`] leaf nodes.
pub trait Blackboard: Default + Clone + Debug {}

impl<T> Blackboard for T where T: Default + Clone + Debug {}

pub trait ActionHandler: Clone {
    type Bb: Blackboard;
    type Execute: Executor<Self::Bb>;
    type Condition: Conditional<Self::Bb>;
}
