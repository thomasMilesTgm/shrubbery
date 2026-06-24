use ahash::HashSet;
use shrubbery::control::control_nodes::ControlNode as CNode;
use shrubbery::control::decorators::StandardDecorator;
use shrubbery::control::ChildUpdate;
use shrubbery::control::ControlTree as CTree;
use shrubbery::control::LeafNode;
use shrubbery::control::ROOT_ID;
use shrubbery::control::{simple_executors::*, CTreeNodeID};
use shrubbery::traits::ExecutorHook;
use shrubbery::Status;

type ControlNode = CNode<StandardDecorator>;
type ControlTree = CTree<StandardDecorator>;

/// [`ExecutorHook`] that returns [`Status::Running`] the first time a node is seen and
/// [`Status::Success`] the second time for testing the behavior of [`ControlNode`] types
/// when leaves are slow to return.
#[derive(Default, Debug, Clone)]
pub struct SlowLeaves {
    pub seen: HashSet<CTreeNodeID>,
    pub logger: LeafLogger,
}

impl ExecutorHook for SlowLeaves {
    /// Returns [`Status::Running`] the first time a node is seen and [`Status::Success`] the
    /// second time.
    fn hook(&mut self, leaf: &LeafNode) -> Status {
        let status = if self.seen.insert(leaf.id.unwrap()) {
            Status::Running
        } else {
            Status::Success
        };
        let mut leaf = leaf.clone();
        leaf.status = Some(status);
        self.logger.hook(&leaf);
        status
    }
}

#[derive(Debug, Clone)]
pub struct FailGiven {
    pub fail_fn: fn(LeafNode) -> Status,
    pub logger: LeafLogger,
}

impl ExecutorHook for FailGiven {
    fn hook(&mut self, leaf: &LeafNode) -> Status {
        let mut leaf = leaf.clone();
        let status = (self.fail_fn)(leaf.clone());
        leaf.status = Some(status);
        self.logger.hook(&leaf);
        status
    }
}

impl FailGiven {
    /// Fails if the [`CTreeNodeID`] inner index even
    pub fn index_is_even() -> Self {
        FailGiven {
            fail_fn: |leaf| (leaf.id.unwrap().index() % 2 != 0).into(),
            logger: LeafLogger::default(),
        }
    }
    /// Fails if the [`CTreeNodeID`] inner index odd.
    pub fn index_is_odd() -> Self {
        FailGiven {
            fail_fn: |leaf| (leaf.id.unwrap().index() % 2 == 0).into(),
            logger: LeafLogger::default(),
        }
    }

    /// Fail unless the index is equal to `INDEX`
    pub fn index_is_not<const A: usize>() -> Self {
        FailGiven {
            fail_fn: |leaf| (A == leaf.id.unwrap().index()).into(),
            logger: LeafLogger::default(),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct AlwaysFail {
    pub logger: LeafLogger,
}

impl ExecutorHook for AlwaysFail {
    fn hook(&mut self, leaf: &LeafNode) -> Status {
        let mut leaf = leaf.clone();
        leaf.status = Some(Status::Failure);
        self.logger.hook(&leaf);
        Status::Failure
    }
}

/// # returns
///
/// `ret = (ControlTree, Vec<CTreeNodeID>)`
///
/// `ret.0`: the [`ControlTree`]
/// `ret.1`: expected order of leaf nodes executed, assuming all succeed
///
/// ```text
///           ( root )
///          /        \
///        (1)        (2)
///      [ --► ]    [ --► ]
///      / / \ \    /  |  \
///     2 3   4 5  7  (8)  9
///                 [ --► ]
///                   / \
///                 10   11
/// ```
///
/// *`[ --► ]` denotes [`ControlNode`] to test
fn test_tree(test: ControlNode) -> (ControlTree, Vec<CTreeNodeID>) {
    test_three(test.clone(), test.clone(), test)
}

/// # returns
///
/// `ret = (ControlTree, Vec<CTreeNodeID>)`
///
/// `ret.0`: the [`ControlTree`]
/// `ret.1`: expected order of leaf nodes executed, assuming all succeed
///
/// ```text
///           ( root )
///          /        \
///        (1)        (2)
///      [ --► ]    [ --► ]
///      / / \ \    /  |  \
///     2 3   4 5  7  (8)  9
///                 [ --► ]
///                   / \
///                 10   11
/// ```
///
/// *`[ --► ]` denotes [`ControlNode`] to test
fn test_three(
    test1: ControlNode,
    test2: ControlNode,
    test8: ControlNode,
) -> (ControlTree, Vec<CTreeNodeID>) {
    let mut control_tree = ControlTree::new();

    let mut expect_leaf_order = vec![];

    /* left branch */

    let seq0_0 = control_tree.add_child(ROOT_ID, test1).unwrap();

    let l0_3 = (0..=3).map(|_| control_tree.add_child(seq0_0, LeafNode::default()).unwrap());

    expect_leaf_order.extend(l0_3);

    /* right branch */
    let seq0_1 = control_tree.add_child(ROOT_ID, test2).unwrap();
    let l4 = control_tree.add_child(seq0_1, LeafNode::default()).unwrap();

    expect_leaf_order.push(l4);

    // sub-sequence
    let seq1_0 = control_tree.add_child(seq0_1, test8).unwrap();
    let l6_7 = (6..=7).map(|_| control_tree.add_child(seq1_0, LeafNode::default()).unwrap());

    expect_leaf_order.extend(l6_7);

    let l8 = control_tree.add_child(seq0_1, LeafNode::default()).unwrap();

    expect_leaf_order.push(l8);
    (control_tree, expect_leaf_order)
}

fn update(status: Status, id: usize) -> ChildUpdate {
    ChildUpdate {
        status,
        child_id: id.into(),
    }
}
fn left_branch(status: Status) -> Vec<ChildUpdate> {
    vec![
        update(status, 2),
        update(status, 3),
        update(status, 4),
        update(status, 5),
    ]
}

fn right_branch_first(status: Status) -> ChildUpdate {
    update(status, 7)
}

fn right_branch_last(status: Status) -> ChildUpdate {
    update(status, 11)
}

fn right_branch_inner(status: Status) -> Vec<ChildUpdate> {
    vec![update(status, 9), update(status, 10)]
}

fn slow_sequence_order() -> Vec<ChildUpdate> {
    let mut seq = left_branch(Status::Running);
    seq.extend(left_branch(Status::Success));
    seq.push(right_branch_first(Status::Running));
    seq.extend(right_branch_inner(Status::Running));
    seq.extend(right_branch_inner(Status::Success));
    seq.push(right_branch_last(Status::Running));
    seq.push(right_branch_first(Status::Success));
    seq.push(right_branch_last(Status::Success));
    seq
}
/// Make sure [`ControlNodeType::Sequence`] executes normally when all it's children are
/// [`Status::Success`]
#[test]
fn happy_sequence() {
    let mut logger = LeafLogger::default();

    let (mut control_tree, expect_leaf_order) = test_tree(ControlNode::sequence());

    let status = control_tree.run(&mut logger);

    assert_eq!(
        status,
        Status::Success,
        "ControlTree should return Status::Success"
    );

    let order_executed = logger
        .updates
        .into_iter()
        .map(|l| l.child_id)
        .collect::<Vec<_>>();

    assert_eq!(order_executed, expect_leaf_order);
}

/// Make sure [`ControlNodeType::Sequence`] continues ticking its children while they are
/// [`Status::Running`]
#[test]
fn slow_sequence() {
    let mut logger = SlowLeaves::default();

    let (mut control_tree, _) = test_tree(ControlNode::sequence());

    let status = control_tree.run(&mut logger);
    // let status = control_tree.run_save_animation(&mut logger, "slow_sequence", 0.75);

    assert_eq!(
        status,
        Status::Success,
        "ControlTree should return Status::Success"
    );

    let updates = logger.logger.updates;

    let expect_updates = slow_sequence_order();
    assert_eq!(
        expect_updates, updates,
        "\n---------- Expected -----------\n\
            {:#?}\n\
            ---------- Found -----------\n\
            {:#?}\n\
            ",
        expect_updates, updates
    );
}

/// Make sure [`ControlNodeType::Sequence`] fails as soon as a child fails
#[test]
fn fail_sequence_fast() {
    let mut logger = AlwaysFail::default();
    let (mut control_tree, _) = test_tree(ControlNode::sequence());

    let status = control_tree.run(&mut logger);

    assert_eq!(
        status,
        Status::Failure,
        "AlwaysFail should return Status::Failure from the full tree"
    );

    assert_eq!(logger.logger.updates.len(), 1);
}

/// Make sure [`ControlNodeType::Parallel`] runs all children regardless of the success or
/// failure.
#[test]
fn slow_parallel() {
    let mut logger = SlowLeaves::default();
    let (mut control_tree, _expect_order) = test_tree(ControlNode::parallel());

    // let status = control_tree.run_save_animation(&mut logger, "slow_parallel", 0.75);
    let status = control_tree.run(&mut logger);

    assert_eq!(
        status,
        Status::Success,
        "Everything should run, but the overall result should be failure"
    );

    let expect_updates = slow_sequence_order();
    let updates = logger.logger.updates;

    assert_eq!(updates, expect_updates);
}

#[test]
fn fallback() {
    const SUCCESS_AT: &[usize] = &[2, 10];
    const FAIL_AT: &[usize] = &[7, 9];
    const NOT_RUN: &[usize] = &[3, 4, 5, 11];

    let mut logger = FailGiven::index_is_odd();

    let (mut control_tree, normal_order) = test_tree(ControlNode::fallback());
    let status = control_tree.run(&mut logger);

    assert_eq!(
        status,
        Status::Success,
        "Fallback should succeed when it hits node #11"
    );

    let updates = logger.logger.updates;
    let expect_updates = normal_order
        .into_iter()
        .flat_map(|id| {
            let status = if FAIL_AT.contains(&id.index()) {
                Some(Status::Failure)
            } else if SUCCESS_AT.contains(&id.index()) {
                Some(Status::Success)
            } else if NOT_RUN.contains(&id.index()) {
                None
            } else {
                panic!("Unexpected leaf node id: {}", id.index());
            }?;
            let update = ChildUpdate {
                child_id: id,
                status,
            };
            Some(update)
        })
        .collect::<Vec<_>>();

    assert_eq!(
        expect_updates, updates,
        "\n---------- Expected -----------\n\
            {:#?}\n\
            ---------- Found -----------\n\
            {:#?}\n\
            ",
        expect_updates, updates
    );
}

#[test]
fn invert() {
    let mut logger = AlwaysFail::default();
    let (mut control_tree, _expect_order) = test_tree(ControlNode::sequence());
    let control_nodes = control_tree
        .iter_control_nodes()
        .filter_map(|n| n.id)
        .collect::<Vec<_>>();

    let parent_ctrl = control_tree
        .iter_tree()
        .flat_map(|(parent, children)| {
            children
                .iter()
                .filter(|c| control_nodes.contains(c))
                .map(|c| (*parent, *c))
        })
        .collect::<Vec<_>>();

    let inverter = ControlNode::decorator(StandardDecorator::inverter());

    parent_ctrl.into_iter().for_each(|(parent, child)| {
        control_tree.insert_between(parent, &[child], inverter.clone());
    });

    let status = control_tree.run(&mut logger);
    // executors are always fail, but they're inverted control_tree.save_dot("ct_invert");
    assert_eq!(status, Status::Success);
}

#[test]
fn repeat() {
    const RETRYS: usize = 3;
    let mut logger = AlwaysFail::default();

    let (mut control_tree, _) = test_tree(ControlNode::parallel());

    let seq = control_tree.insert_between(
        ROOT_ID,
        &control_tree.children(&ROOT_ID),
        ControlNode::sequence(),
    );

    control_tree.insert_between(ROOT_ID, &[seq], ControlNode::repeater(RETRYS));

    let status = control_tree.run(&mut logger);
    // let status = control_tree.run_save_animation(&mut logger, "repeat", 0.75);

    let mut expected_updates = left_branch(Status::Failure).clone();

    let exp = expected_updates.clone();

    for _ in 0..RETRYS {
        expected_updates.extend(exp.clone());
    }

    let updates = logger.logger.updates;

    assert_eq!(updates, expected_updates);
    assert_eq!(status, Status::Failure);
}

#[test]
fn nested_repeat() {
    const RETRIES: usize = 4;
    let mut logger = AlwaysFail::default();

    let (mut control_tree, _) = test_tree(ControlNode::parallel());

    let seq = control_tree.insert_between(
        ROOT_ID,
        &control_tree.children(&ROOT_ID),
        ControlNode::sequence(),
    );

    let repeat_1 = control_tree.insert_between(ROOT_ID, &[seq], ControlNode::repeater(RETRIES));

    control_tree.insert_between(ROOT_ID, &[repeat_1], ControlNode::repeater(RETRIES));

    // let status = control_tree.run_save_animation(&mut logger, "repeat_squared", 0.1);
    let status = control_tree.run(&mut logger);

    let mut expected_updates = vec![];

    let exp = left_branch(Status::Failure);

    for _ in 0..(RETRIES + 1) {
        for _ in 0..(RETRIES + 1) {
            expected_updates.extend(exp.clone());
        }
    }

    let updates = logger.logger.updates;

    assert_eq!(
        updates,
        expected_updates,
        "Expected {} updates, got {}",
        expected_updates.len(),
        updates.len()
    );
    assert_eq!(status, Status::Failure);
}
