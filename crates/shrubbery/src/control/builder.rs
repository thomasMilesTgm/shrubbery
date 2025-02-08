/* Copyright (C) 2023 Admix Pty. Ltd. - All Rights Reserved.
Unauthorized copying of this file, via any medium is strictly prohibited.
Proprietary and confidential. */

use crate::ShrubberyResult;

use super::control_nodes::*;
use super::*;
use decorators::*;

/// For building control trees
#[derive(Clone)]
pub struct CTreeBuilder<D: Decorator> {
    pub(crate) inner: ControlTree<D>,
}

impl<D: Decorator> ControlTree<D> {
    pub fn into_builder(self) -> CTreeBuilder<D> {
        CTreeBuilder::from(self)
    }
    pub fn builder() -> CTreeBuilder<D> {
        CTreeBuilder::new()
    }
}

impl<D: Decorator> From<ControlTree<D>> for CTreeBuilder<D> {
    fn from(value: ControlTree<D>) -> Self {
        CTreeBuilder { inner: value }
    }
}

impl<D: Decorator> Default for CTreeBuilder<D> {
    fn default() -> Self {
        Self::new()
    }
}

pub type CTreeLayerFn<O, D> = fn(CTreeLayerBuilder<'_, D>) -> O; // -> LayerBuilder<'_, D>;

impl<D: Decorator> CTreeBuilder<D> {
    pub fn new() -> Self {
        Self {
            inner: ControlTree::new(),
        }
    }
    pub fn layer<O>(&mut self, f: CTreeLayerFn<O, D>) -> O {
        f(CTreeLayerBuilder {
            builder: self,
            layer_id: ROOT_ID,
            layer_depth: 0,
        })
    }

    /// Build the [`ShrubberyBT`]
    ///
    /// # Errors
    ///
    /// - If a cycle is detected in the tree
    /// - If a control node is left dangling (missing leaf)
    /// - If a decorator has more than one child
    pub fn build(self) -> ShrubberyResult<ControlTree<D>> {
        // validate the tree
        self.inner.validate_bt_rules()?;

        Ok(self.inner)
    }

    /// Inject a cycle. This will make [`Self::build`] fail, so don't use it unless you're testing
    /// that.
    #[cfg(test)]
    pub fn inject_cycle(&mut self) {
        let parent = ROOT_ID;
        let child = self
            .inner
            .add_child_unchecked(parent, ControlNode::sequence());
        self.inner.tree.entry(child).or_default().push(parent);
    }
}

pub struct CTreeLayerBuilder<'a, D: Decorator> {
    pub(crate) builder: &'a mut CTreeBuilder<D>,
    pub layer_id: CTreeNodeID,
    pub layer_depth: usize,
}

impl<'a, D: Decorator> CTreeLayerBuilder<'a, D> {
    /// create a new layer builder with depth 0
    pub fn new(
        builder: &'a mut CTreeBuilder<D>,
        layer_id: CTreeNodeID,
    ) -> CTreeLayerBuilder<'a, D> {
        Self {
            builder,
            layer_id,
            layer_depth: 0,
        }
    }

    /// Add a [`Sequence`] node, and build it's sub-tree
    pub fn sequence<O>(&mut self, layer_fn: CTreeLayerFn<O, D>) -> O {
        self.control_node(ControlNode::sequence(), layer_fn)
    }

    /// Add a [`Fallback`] node, and build it's sub-tree
    pub fn fallback<O>(&mut self, layer_fn: CTreeLayerFn<O, D>) -> O {
        self.control_node(ControlNode::fallback(), layer_fn)
    }

    /// Add a [`Parallel`] node, and build it's sub-tree
    pub fn parallel<O>(&mut self, layer_fn: CTreeLayerFn<O, D>) -> O {
        self.control_node(ControlNode::parallel(), layer_fn)
    }

    pub fn decorator<O>(&mut self, decorator: impl Into<D>, layer_fn: CTreeLayerFn<O, D>) -> O {
        let node = ControlNode::decorator(decorator.into());
        self.control_node(node, layer_fn)
    }

    pub fn map<O>(&mut self, f: fn(&mut Self) -> O) -> O {
        f(self)
    }

    pub fn next_layer(&mut self, node: impl Into<ControlNode<D>>) -> CTreeLayerBuilder<'_, D> {
        let parent_id = self
            .builder
            .inner
            .add_child_unchecked(self.layer_id, node.into());

        CTreeLayerBuilder {
            builder: self.builder,
            layer_id: parent_id,
            layer_depth: self.layer_depth + 1,
        }
    }

    /// `layer_fn` provides a [`LayerBuilder`] for defining the subtree beneath the added `node`
    pub fn control_node<O>(
        &mut self,
        node: impl Into<ControlNode<D>>,
        layer_fn: CTreeLayerFn<O, D>,
    ) -> O {
        let layer_builder = self.next_layer(node);
        layer_fn(layer_builder)
    }

    /// Add a leaf node.
    ///
    /// NOTE: If you're using this via the [`std::ops::Deref`] implementation on
    /// [`BTLayerBuider`](crate::bt::builder::BTLayerBuidler), you probably should be using the
    /// [`execute`](crate::bt::builder::BTLayerBuidler::execute) or
    /// [`condition`](crate::bt::builder::BTLayerBuidler::condition) method instead, otherwise the
    /// dispatch will not be created & the behavior will not actually run!
    pub fn leaf_node(&mut self, node: impl Into<LeafNode>) -> CTreeNodeID {
        self.builder
            .inner
            .add_child_unchecked(self.layer_id, node.into())
    }
}

impl<'a, D: Decorator + From<StandardDecorator>> CTreeLayerBuilder<'a, D> {
    pub fn repeat<O>(&mut self, retries: usize, layer_fn: CTreeLayerFn<O, D>) -> O {
        let decorator = D::from(Repeater::new(retries).into());
        self.decorator(decorator, layer_fn)
    }
    pub fn invert<O>(&mut self, layer_fn: CTreeLayerFn<O, D>) -> O {
        let decorator = D::from(Inverter::default().into());
        self.decorator(decorator, layer_fn)
    }

    pub fn subtree_named<O>(&mut self, name: &str, layer_fn: CTreeLayerFn<O, D>) -> O {
        let decorator = D::from(Subtree::new(name.to_string()).into());
        self.decorator(decorator, layer_fn)
    }
    pub fn subtree<O>(&mut self, layer_fn: CTreeLayerFn<O, D>) -> O {
        let decorator = D::from(Subtree::default().into());
        self.decorator(decorator, layer_fn)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn cyclic_nobuild() {
        let mut builder = ControlTree::<StandardDecorator>::builder();

        builder.layer(|mut root| {
            root.sequence(|mut sequence| {
                sequence.leaf_node(LeafNode::default());
                sequence.leaf_node(LeafNode::default());
            });
        });

        builder.inject_cycle();

        let err = builder.build().unwrap_err();

        assert!(matches!(err, ShrubberyError::CycleDetected(_)));
    }

    #[test]
    fn dangling_nobuild() {
        let mut builder = ControlTree::<StandardDecorator>::builder();
        builder.layer(|mut root_layer| {
            root_layer.leaf_node(LeafNode::default());
        });

        builder.layer(|root_layer| {
            root_layer
                .builder
                .inner
                // the unchecked version is only crate-public so you normally can't do this, even
                // if you end up with &mut ControlTree.
                .add_child_unchecked(root_layer.layer_id, ControlNode::sequence());
        });

        let err = builder.build().unwrap_err(); // panic if this isn't an error

        assert!(matches!(err, ShrubberyError::DanglingControlNode(_)));
    }

    #[test]
    fn multiple_decorator_children_nobuild() {
        let mut builder = ControlTree::<StandardDecorator>::builder();
        builder.layer(|mut root_layer| {
            // inverter is a decorator, not allowed to have multiple children
            root_layer.decorator(StandardDecorator::inverter(), |mut decorator_layer| {
                // two leaf node children
                decorator_layer.leaf_node(LeafNode::default());
                decorator_layer.leaf_node(LeafNode::default());
            });
        });
        let err = builder.build().unwrap_err(); // panic if this isn't an error

        assert!(matches!(err, ShrubberyError::InvalidDecorator { .. }));
    }
}
