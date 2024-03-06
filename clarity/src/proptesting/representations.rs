use proptest::prelude::*;

use crate::vm::{representations::{Span, TraitDefinition}, ClarityName, ContractName, SymbolicExpression, SymbolicExpressionType};

use super::*;

pub fn clarity_name() -> impl Strategy<Value = ClarityName> {
    "[a-z]{40}".prop_map(|s| s.try_into().unwrap())
}

pub fn contract_name() -> impl Strategy<Value = ContractName> {
    "[a-zA-Z]{1,40}".prop_map(|s| s.try_into().unwrap())
}

pub fn trait_definition() -> impl Strategy<Value = TraitDefinition> {
    prop_oneof![
        trait_identifier().prop_map(TraitDefinition::Defined),
        trait_identifier().prop_map(TraitDefinition::Imported)
    ]
}

pub fn symbolic_expression() -> impl Strategy<Value = SymbolicExpression> {
    (
        symbolic_expression_type(),
        0u64..u64::MAX,
        Just(Span::zero()),
        Just(Vec::<(String, Span)>::new()),
        Just(None::<String>),
        Just(Vec::<(String, Span)>::new()),
    )
    .prop_map(|(expr, id, span, pre_comments, end_line_comment, post_comments)| 
        SymbolicExpression {
            expr,
            id,
            #[cfg(feature = "developer-mode")]
            span,
            #[cfg(feature = "developer-mode")]
            pre_comments,
            #[cfg(feature = "developer-mode")]
            end_line_comment,
            #[cfg(feature = "developer-mode")]
            post_comments,
        }
    )
}

pub fn symbolic_expression_type() -> impl Strategy<Value = SymbolicExpressionType> {
    prop_oneof![
        // Atom
        clarity_name().prop_map(SymbolicExpressionType::Atom),
        // AtomValue
        PropValue::any().prop_map(|val| SymbolicExpressionType::AtomValue(val.into())),
        // LiteralValue
        PropValue::any().prop_map(|val| SymbolicExpressionType::LiteralValue(val.into())),
        // Field
        trait_identifier().prop_map(SymbolicExpressionType::Field),
        // TraitReference
        (clarity_name(), trait_definition())
            .prop_map(|(n, t)| SymbolicExpressionType::TraitReference(n, t)),
    ]
}