use proptest::prelude::*;
use proptest::string::string_regex;

use crate::types::{StacksHashMap, StacksHashSet};
use crate::vm::contracts::Contract;
use crate::vm::types::{
    ASCIIData, BuffData, CharType, ListData, ListTypeData, OptionalData, PrincipalData,
    QualifiedContractIdentifier, ResponseData, SequenceData, SequenceSubtype,
    StandardPrincipalData, StringSubtype, StringUTF8Length, TupleData, TupleTypeSignature,
    TypeSignature, UTF8Data, Value, MAX_VALUE_SIZE, TraitIdentifier
};
use crate::vm::callables::{DefinedFunction, FunctionIdentifier, DefineType};
use crate::vm::{ClarityName, ClarityVersion, ContractContext, ContractName};
use crate::vm::representations::{Span, SymbolicExpression, SymbolicExpressionType, TraitDefinition};

use super::*;

pub fn standard_principal_data() -> impl Strategy<Value = StandardPrincipalData> {
    (
        0u8..32, 
        prop::collection::vec(any::<u8>(), 20)
    )
    .prop_map(|(v, hash)| 
        StandardPrincipalData(v, hash.try_into().unwrap())
    )
}

pub fn qualified_contract_identifier() -> impl Strategy<Value = QualifiedContractIdentifier> {
    (
        standard_principal_data(),
        contract_name()
    )
    .prop_map(|(issuer, name)| 
        QualifiedContractIdentifier {
            issuer,
            name
        }
    )
}

pub fn trait_identifier() -> impl Strategy<Value = TraitIdentifier> {
    (
        clarity_name(),
        qualified_contract_identifier()
    )
    .prop_map(|(name, contract_identifier)| 
        TraitIdentifier {
            name,
            contract_identifier
        }
    )
}

pub fn type_signature() -> impl Strategy<Value = TypeSignature> {
    let leaf = prop_oneof![
        Just(TypeSignature::IntType),
        Just(TypeSignature::UIntType),
        Just(TypeSignature::BoolType),
        (0u32..128).prop_map(|s| TypeSignature::SequenceType(SequenceSubtype::BufferType(
            s.try_into().unwrap()
        ))),
        (0u32..128).prop_map(|s| TypeSignature::SequenceType(SequenceSubtype::StringType(
            StringSubtype::ASCII(s.try_into().unwrap())
        ))),
        Just(TypeSignature::PrincipalType),
        (0u32..32).prop_map(|s| TypeSignature::SequenceType(SequenceSubtype::StringType(
            StringSubtype::UTF8(s.try_into().unwrap())
        )))
    ];
    
    leaf.prop_recursive(3, 32, 5, |inner| prop_oneof![
        // optional type: 10% NoType + 90% any other type
        prop_oneof![
            1 => Just(TypeSignature::NoType),
            9 => inner.clone(),
        ]
        .prop_map(|t| TypeSignature::new_option(t).unwrap()),
        // response type: 20% (NoType, any) + 20% (any, NoType) + 60% (any, any)
        prop_oneof![
            1 => inner.clone().prop_map(|ok_ty| TypeSignature::new_response(ok_ty, TypeSignature::NoType).unwrap()),
            1 => inner.clone().prop_map(|err_ty| TypeSignature::new_response(TypeSignature::NoType, err_ty).unwrap()),
            3 => (inner.clone(), inner.clone()).prop_map(|(ok_ty, err_ty)| TypeSignature::new_response(ok_ty, err_ty).unwrap()),
        ],
        // tuple type
        prop::collection::btree_map(
            r#"[a-zA-Z]{1,16}"#.prop_map(|name| name.try_into().unwrap()),
            inner.clone(),
            1..8
        )
        .prop_map(|btree| TypeSignature::TupleType(btree.try_into().unwrap())),
        // list type
        (1u32..8, inner.clone()).prop_map(|(s, ty)| (ListTypeData::new_list(ty, s).unwrap()).into()),
    ])
}

