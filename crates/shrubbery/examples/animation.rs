/* Copyright (C) 2023 Admix Pty. Ltd. - All Rights Reserved.
Unauthorized copying of this file, via any medium is strictly prohibited.
Proprietary and confidential. */

use derive_more::From;
use shrubbery::prelude::*;

#[derive(Debug, Default, Clone, Copy)]
struct BB {
    sub: SubBB,
}

#[derive(Debug, Default, Clone, Copy)]
struct SubBB;

impl AsMut<SubBB> for BB {
    fn as_mut(&mut self) -> &mut SubBB {
        &mut self.sub
    }
}

#[derive(Debug, Clone, From)]
enum Executors {
    E0(Execute0Fails),
    E1(Execute1Succeeds),
    E2(Execute2Succeeds),
    Sub(SubExecutors),
}

#[derive(Debug, Default, Clone, Copy)]
struct Execute0Fails;

#[derive(Debug, Default, Clone, Copy)]
struct Execute1Succeeds;

#[derive(Debug, Default, Clone, Copy)]
struct Execute2Succeeds;

#[derive(Debug, Clone, From)]
enum SubExecutors {
    Sub0(SubExec0Fails),
    Sub1(SubExec1Succeeds),
}

#[derive(Debug, Default, Clone, Copy)]
struct SubExec0Fails;

#[derive(Debug, Default, Clone, Copy)]
struct SubExec1Succeeds;

impl Executor<BB> for Execute0Fails {
    fn execute(&self, _: &mut BB) -> Status {
        println!("Execute0");
        Status::Failure
    }
    fn name(&self) -> Option<String> {
        Some("Execute0".to_string())
    }
}
impl Executor<BB> for Execute1Succeeds {
    fn execute(&self, _: &mut BB) -> Status {
        println!("Execute1");
        Status::Success
    }
    fn name(&self) -> Option<String> {
        Some("Execute1".to_string())
    }
}
impl Executor<BB> for Execute2Succeeds {
    fn execute(&self, _: &mut BB) -> Status {
        println!("Execute2");
        Status::Success
    }
    fn name(&self) -> Option<String> {
        Some("Execute2".to_string())
    }
}

impl Executor<SubBB> for SubExec0Fails {
    fn execute(&self, _: &mut SubBB) -> Status {
        println!("SubExec0");
        Status::Failure
    }
    fn name(&self) -> Option<String> {
        Some("SubExec0".to_string())
    }
}

impl Executor<SubBB> for SubExec1Succeeds {
    fn execute(&self, _: &mut SubBB) -> Status {
        println!("SubExec1");
        Status::Success
    }
    fn name(&self) -> Option<String> {
        Some("SubExec1".to_string())
    }
}

impl Executor<SubBB> for SubExecutors {
    fn execute(&self, bb: &mut SubBB) -> Status {
        match self {
            SubExecutors::Sub0(e) => e.execute(bb),
            SubExecutors::Sub1(e) => e.execute(bb),
        }
    }
    fn name(&self) -> Option<String> {
        match self {
            SubExecutors::Sub0(e) => e.name(),
            SubExecutors::Sub1(e) => e.name(),
        }
    }
}

impl Executor<BB> for Executors {
    fn execute(&self, bb: &mut BB) -> Status {
        match self {
            Executors::E0(e) => e.execute(bb),
            Executors::E1(e) => e.execute(bb),
            Executors::E2(e) => e.execute(bb),
            Executors::Sub(e) => e.execute(bb.as_mut()),
        }
    }
    fn name(&self) -> Option<String> {
        match self {
            Executors::E0(e) => e.name(),
            Executors::E1(e) => e.name(),
            Executors::E2(e) => e.name(),
            Executors::Sub(e) => e.name(),
        }
    }
}

#[derive(Debug, Clone, From)]
enum Conditions {
    C0(Condition0),
    Sub(SubConditions),
}

#[derive(Debug, Clone, From)]
enum SubConditions {
    Sub0(SubCondition0Fails),
    Sub1(SubCondition1),
}

impl Conditional<BB> for Conditions {
    fn conditional(&self, bb: &BB) -> Status {
        match self {
            Conditions::C0(c) => c.conditional(bb),
            Conditions::Sub(c) => c.conditional(bb),
        }
    }
    fn name(&self) -> Option<String> {
        match self {
            Conditions::C0(c) => c.name(),
            Conditions::Sub(c) => c.name(),
        }
    }
}

impl Conditional<BB> for SubConditions {
    fn conditional(&self, bb: &BB) -> Status {
        match self {
            SubConditions::Sub0(c) => c.conditional(bb),
            SubConditions::Sub1(c) => c.conditional(bb),
        }
    }
    fn name(&self) -> Option<String> {
        match self {
            SubConditions::Sub0(c) => c.name(),
            SubConditions::Sub1(c) => c.name(),
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct Condition0;

impl Conditional<BB> for Condition0 {
    fn conditional(&self, _: &BB) -> Status {
        Status::Success
    }
    fn name(&self) -> Option<String> {
        Some("Condition0".to_string())
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct SubCondition0Fails;

impl Conditional<BB> for SubCondition0Fails {
    fn conditional(&self, _: &BB) -> Status {
        Status::Failure
    }
    fn name(&self) -> Option<String> {
        Some("SubCondition0".to_string())
    }
}
#[derive(Debug, Default, Clone, Copy)]
struct SubCondition1;

impl Conditional<BB> for SubCondition1 {
    fn conditional(&self, _: &BB) -> Status {
        Status::Success
    }
    fn name(&self) -> Option<String> {
        Some("SubCondition1".to_string())
    }
}

#[derive(Debug, Default, Clone)]
struct TestHandler;

impl ActionHandler for TestHandler {
    type Bb = BB;
    type Execute = Executors;
    type Condition = Conditions;
}

type TestBTBuilder = BTBuilder<TestHandler, StandardDecorator>;
type TestLayerBuilder<'a> = BTLayer<'a, TestHandler, StandardDecorator>;
fn main() {
    fn sequence(mut layer_root: TestLayerBuilder<'_>) -> TestLayerBuilder<'_> {
        layer_root.sequence(|mut seq| {
            seq.condition(SubConditions::Sub0(SubCondition0Fails));
            seq.execute(SubExecutors::Sub0(SubExec0Fails));
            seq.condition(SubConditions::Sub1(SubCondition1));
        });
        layer_root
    }

    fn parallel(mut layer_root: TestLayerBuilder<'_>) -> TestLayerBuilder<'_> {
        layer_root.parallel(|mut ep| {
            ep.condition(Condition0);
            ep.execute(Execute0Fails);
            ep.execute(Execute1Succeeds);
        });
        layer_root
    }

    fn inverter(mut layer_root: TestLayerBuilder<'_>) -> TestLayerBuilder<'_> {
        layer_root.inverter(|mut ep| {
            ep.condition(SubConditions::Sub0(SubCondition0Fails));
        });
        layer_root
    }

    fn repeater(mut layer_root: TestLayerBuilder<'_>) -> TestLayerBuilder<'_> {
        layer_root.repeater(3, |mut ep| {
            ep.fallback(|mut ep| {
                ep.execute(Execute0Fails);
                ep.execute(SubExecutors::Sub0(SubExec0Fails));
            });
        });
        layer_root
    }

    let mut builder = TestBTBuilder::default();
    builder.layer(|mut root_layer| {
        root_layer.fallback(|fallback| {
            fallback
                .map(parallel)
                .map(sequence)
                .map(repeater)
                .map(inverter);
        })
    });

    let mut bt = builder.build().unwrap();

    bt.save_dot("bt_builder");
    let mut bb = BB::default();
    bt.run_save_animation(&mut bb, "bt_builder", 0.5);
}
