/* Copyright (C) 2023 Admix Pty. Ltd. - All Rights Reserved.
Unauthorized copying of this file, via any medium is strictly prohibited.
Proprietary and confidential. */

//! # Shrubbery
//!
//! <img src="../../../bt/crates/shrubbery/doc/shrub-dark.gif" alt="sample bt" />
//!
//! <img src="../../../bt/crates/shrubbery/doc/shrubbery.jpg" alt="it is a good shrubbery" />
//!
//! ## Motivation
//!
//! Fundamentally behavior trees are _simple_, leaf nodes perform some action and return one of
//! three states: Success, Failure, or Running. All other nodes nodes determine how tree traversal
//! is done -- effectively controlling which execution nodes get executed and in what order.
//!
//! ## BT vs FSM
//!
//! FSMs are more explicit in their state transitions, but can be more difficult to manage as the
//! number of states grows, especially when it comes to modification of behavior. In contrast,
//! BTs are more flexible and can be more easily modified, since behavior is localized strictly
//! to the sub-tree where the behavior is defined, changes to a sub-tree do not impact the program
//! outside of that sub-tree.
//!
//! The trade offs are:
//!
//! - Reactivity: FSMs are more reactive, since state transitions are explicit, in a BT the
//!   execution flow is essentially compiled into the shape of the tree.
//!
//! ## Motivation for Shrubbery (why not keep using `bonsai_bt`)
//!
//! IMO bonsai has a few insurmountable issues that make it unsuitable:
//!
//! - API: The way the BT gets defined is by constructing a single, ultra-nested enum of
//!   `Behavior<T>` upfront that cannot be changed once the `BT` is instantiated. This is both
//!   cumbersome to and error prone to write (a problem I tried to mitigate in `bt_functional`), and
//!   more importantly, very difficult to compose subtrees with different `T` into a greater
//!   behavior tree and there is no mechanism for helping with this
//!
//! **Node seperation**
//!
//! Control (internal) nodes and Executor (leaf) nodes are not delineated (they are all `Behavior<T>`).
//!
//! - To `&mut` or not to `&mut`: typically execution nodes have two flavors:
//!     - Read a value from the blackboard and return `Success` if it's present, `Failure` if
//!       it's not, or `Running` to indicate a loop should continue executing.
//!     - Execute a task & update the blackboard with the outcome, return `Success`, `Failure`,
//!       or `Running` to reflect the outcome of the task.
//!     - In bonsai, you always have an `&mut Blackboard` when handling an action in a tick and
//!       the read vs. execute and write distinction doesn't exist, this makes it tempting to do
//!       work in places that it's not appropriate.
//!
//! - If Control is independent of Execution, `T` (the executable behavior type) is independent
//!   of Control, thus by building the `BT` out of indices into a `Vec<ControlNode>` and a
//!   `Vec<ExecutorNode>` (which can be done implicitly since nodes are executors iff they are
//!   a leaf, and nodes are control iff they are not executors). Seperating these in this way
//!   makes sub-tree composition trivially easy.
//!
//! **Graphviz**
//!
//! This feels like a nit, but I don't think it is. A lot of the debugging behavior trees is made a
//! lot easier with the ability to look at the tree itself.
//!
//! A nicely formatted graphviz tree goes a long way to make it easy to parse what is going on &
//! there are a standard set of symbols typically used to do so:
//!
//! <img src="../../../bt/crates/shrubbery/doc/sample_bt.png" alt="sample bt" />
//!
//! Petgraphs' `Dot` implementation isn't great since you can't style nodes differently and it relies
//! purely on the implementation of [`std::fmt::Debug`] (something very useless if your nodes are ids),
//! but at least you can use [`grahviz-rust`] & tree traveral to DIY it.
//!
//! Bonsai uses petgraph, but doens't expose the actual graph in the public API, they just wrap the
//! [`petgraph::Dot`] implementation and return a string. We could vendor `bonsai` or try and land a
//! PR to get around this, but the other issues I have mean I really don't care.
//!
//!

use std::fmt::Debug;

use control::CTreeNodeID;
use thiserror::Error;

pub mod bt;
pub mod control;
pub mod executor_mask;
pub mod graphviz;
pub mod traits;

#[cfg(test)]
pub mod null_types;

pub mod prelude {
    pub use crate::bt::builder::*;
    pub use crate::bt::ShrubberyBT;
    pub use crate::control::control_nodes::*;
    pub use crate::control::decorators::*;
    pub use crate::control::simple_executors::LeafLogger;
    pub use crate::control::ControlTree;
    pub use crate::control::LeafNode;
    pub use crate::control::RootNode;
    pub use crate::control::StdControlTree;
    pub use crate::traits::*;

    pub use crate::{ShrubberyError, ShrubberyResult, Status};
}

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ShrubberyError {
    #[error("ShrubberyError: Cycle detected: {0:?}")]
    CycleDetected(Vec<CTreeNodeID>),

    #[error("ShrubberyError: Dangling control node: {0:?}")]
    DanglingControlNode(CTreeNodeID),

    #[error("\
        ShrubberyError: Decorator must have exactly one child, found {}.\n\
        {decorator:?} -> {children:?}", children.len())]
    InvalidDecorator {
        decorator: CTreeNodeID,
        children: Vec<CTreeNodeID>,
    },
}

pub type ShrubberyResult<T> = Result<T, ShrubberyError>;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Status {
    /// Node succeeded
    Success,

    /// Node failed
    Failure,

    /// Node is still running. In the case of control nodes, this means that execution flow to
    /// continue to execute in it's children.
    #[default]
    Running,
}

impl Status {
    pub fn into_failure_if_running(self) -> Self {
        if self.is_running() {
            Status::Failure
        } else {
            self
        }
    }
    pub fn is_terminal(&self) -> bool {
        self.is_success() || self.is_failure()
    }

    pub fn is_running(&self) -> bool {
        matches!(self, Status::Running)
    }
    pub fn is_success(&self) -> bool {
        matches!(self, Status::Success)
    }
    pub fn is_failure(&self) -> bool {
        matches!(self, Status::Failure)
    }
}

impl std::ops::Not for Status {
    type Output = Self;
    /// Invert the status.
    ///
    /// NOTE: !Running == Running.. Is this confusing?
    ///
    /// |    | Success | Failure | Running |
    /// | -- | ------- | ------- | -------
    /// | !  | Failure | Success | Running |
    fn not(self) -> Self {
        match self {
            Status::Success => Status::Failure,
            Status::Failure => Status::Success,
            Status::Running => Status::Running,
        }
    }
}

impl From<bool> for Status {
    fn from(val: bool) -> Self {
        if val {
            Status::Success
        } else {
            Status::Failure
        }
    }
}
