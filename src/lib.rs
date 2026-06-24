#![doc = include_str!("../README.md")]

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
