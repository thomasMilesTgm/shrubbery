use super::ChildUpdate;
use super::LeafNode;
use crate::traits::*;
use crate::Status;

/// Logs the ids of and statuses returned by leaf nodes in the order they were executed in.
///
/// Can be useful to include as part of executors that do more complex stuff.
#[derive(Default, Debug, Clone)]
pub struct LeafLogger {
    /// Only contains [`LeafNode`] [`ChildUpdate`] values (i.e. No [`ChildUpdate`] values from
    /// [`ControlNode`]/[`RootNode`] ticks).
    pub updates: Vec<ChildUpdate>,
}

impl ExecutorHook for LeafLogger {
    fn hook(&mut self, leaf: &LeafNode) -> Status {
        let status = leaf.status.unwrap_or(Status::Success);
        self.updates.push(ChildUpdate {
            status,
            child_id: leaf.id.unwrap(),
        });
        status
    }
}
