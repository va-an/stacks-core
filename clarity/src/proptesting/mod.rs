use proptest::{prop_oneof, strategy::{Just, Strategy, ValueTree}, test_runner::{Config, RngAlgorithm, TestRng, TestRunner}};
use rand::Rng;
use stacks_common::types::StacksHashMap as HashMap;

pub mod types;
pub mod values;
pub mod callables;
pub mod representations;
pub mod contracts;

pub use types::*;
pub use values::*;
pub use callables::*;
pub use representations::*;
pub use contracts::*;

pub fn clarity_version() -> impl Strategy<Value = crate::vm::ClarityVersion> {
    prop_oneof![
        Just(crate::vm::ClarityVersion::Clarity1),
        Just(crate::vm::ClarityVersion::Clarity2),
    ]
}