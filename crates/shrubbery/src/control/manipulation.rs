/* Copyright (C) 2023 Admix Pty. Ltd. - All Rights Reserved.
Unauthorized copying of this file, via any medium is strictly prohibited.
Proprietary and confidential. */

use ahash::HashMap;

use super::{CTreeNode, CTreeNodeID, ControlNode, ControlTree};
use crate::prelude::*;

impl<D: Decorator + From<StandardDecorator>> ControlTree<D> {
    pub fn add_subtree_as_last_child(&mut self, from: CTreeNodeID, subtree: impl Into<Self>) {
        self.add_subtree_with_priority(from, usize::MAX, subtree)
    }

    pub fn add_subtree_as_first_child(&mut self, from: CTreeNodeID, subtree: impl Into<Self>) {
        self.add_subtree_with_priority(from, 0, subtree)
    }

    /// Add a subtree below the node at `from`, with a `priority` value (the position in the
    /// left->right order of the tree). The priority is simply the index in the children vector in
    /// [`Self::tree`], and the bt runs from `0..tree.len()`.
    pub fn add_subtree_with_priority(
        &mut self,
        from: CTreeNodeID,
        priority: usize,
        subtree: impl Into<Self>,
    ) {
        //
        let subtree_root =
            self.add_floating_node(ControlNode::decorator(StandardDecorator::subtree()));

        let siblings = self.tree.entry(from).or_default();
        let index = priority.min(siblings.len());
        siblings.insert(index, subtree_root);

        let ControlTree { nodes, tree } = subtree.into();
        let mut old_to_new = HashMap::default();

        nodes.into_iter().filter(|n| !n.is_root()).for_each(|node| {
            let old_id = node.id().unwrap();
            let new_id = self.add_floating_node(node);
            old_to_new.insert(old_id, new_id);
        });

        tree.into_iter()
            .filter(|(p, _)| old_to_new.contains_key(p)) // skip the root
            .for_each(|(old_parent, children)| {
                // add new child ids to self
                let new_children = children
                    .into_iter()
                    .flat_map(|old_child| old_to_new.get(&old_child));

                self.tree
                    .entry(old_parent)
                    .or_default()
                    .extend(new_children);
            });
    }
}

impl<D: Decorator> ControlTree<D> {
    /// Add a new node as a child with a priority (0 runs first).
    pub fn add_child(
        &mut self,
        parent_id: CTreeNodeID,
        child: impl Into<CTreeNode<D>>,
    ) -> ShrubberyResult<CTreeNodeID> {
        self.add_child_with_priority(parent_id, child, usize::MAX)
    }

    /// Add a new node as a child with a priority (0 runs first).
    pub fn add_child_with_priority(
        &mut self,
        parent_id: CTreeNodeID,
        child: impl Into<CTreeNode<D>>,
        priority: usize,
    ) -> ShrubberyResult<CTreeNodeID> {
        let id = self.add_child_unchecked_with_priority(parent_id, child, priority);

        self.recurse_children_check_cycles(parent_id, vec![])
            .map(|_| id)
            .map_err(|e| {
                self.remove(id);
                e
            })
    }

    /// Adds a child node to the root of the tree.
    ///
    /// ## UNCHECKED
    ///
    /// XXX: You are free to break the tree condition using this method -- if you're running into
    /// infinite loops, this is likely the cause.
    pub(crate) fn add_child_unchecked(
        &mut self,
        parent_id: CTreeNodeID,
        child: impl Into<CTreeNode<D>>,
    ) -> CTreeNodeID {
        self.add_child_unchecked_with_priority(parent_id, child, usize::MAX)
    }
    /// Adds a child node to the root of the tree with a givin priority (0 runs first).
    ///
    /// ## UNCHECKED
    ///
    /// XXX: You are free to break the tree condition using this method -- if you're running into
    /// infinite loops, this is likely the cause.
    pub(crate) fn add_child_unchecked_with_priority(
        &mut self,
        parent_id: CTreeNodeID,
        child: impl Into<CTreeNode<D>>,
        priority: usize,
    ) -> CTreeNodeID {
        let mut child = child.into();
        let id: CTreeNodeID = if let Some(id) = child.id() {
            id
        } else {
            self.nodes.len().into()
        };
        child.set_id(id);

        self.nodes.insert(id.0, child);

        let siblings = self.tree.entry(parent_id).or_default();
        let index = priority.min(siblings.len());
        siblings.insert(index, id);

        self.tree.entry(id).or_default();
        id
    }

    /// Remove a node and all references to it from the tree (does not remove the actual node from
    /// [`Self::nodes`], so you can put it back by ID without re-inserting the node).
    pub fn remove(&mut self, id: CTreeNodeID) {
        self.tree.values_mut().for_each(|v| v.retain(|c| c != &id));
        self.tree.remove(&id);
    }
}
