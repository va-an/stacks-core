use proptest::prelude::*;
use rand::distributions::uniform::SampleRange;
use serde::de::value;

use crate::vm::{callables::{DefineType, DefinedFunction, FunctionIdentifier}, database::{DataMapMetadata, DataVariableMetadata, FungibleTokenMetadata, NonFungibleTokenMetadata}, representations::TraitDefinition, types::FunctionSignature};

use super::*;

/// Returns a [`Strategy`] for randomly generating a [`FunctionIdentifier`] instance
/// representing a user-defined function.
pub fn function_identifier_user() -> impl Strategy<Value = FunctionIdentifier> {
    (
        clarity_name(),
        clarity_name()
    )
    .prop_map(|(name, context)| 
        FunctionIdentifier::new_user_function(&context.to_string(), &name.to_string())
    )
}

/// Returns a [`Strategy`] for randomly generating a [`FunctionIdentifier`] instance
/// representing a native function.
pub fn function_identifier_native() -> impl Strategy<Value = FunctionIdentifier> {
    (
        clarity_name()
    )
    .prop_map(|name| 
        FunctionIdentifier::new_native_function(&name.to_string())
    )
}

/// Returns a [`Strategy`] for randomly generating a [`FunctionIdentifier`]
/// instance representing a function of any kind, user-defined or native.
pub fn function_identifier() -> impl Strategy<Value = FunctionIdentifier> {
    prop_oneof![
        function_identifier_user(),
        function_identifier_native()
    ]
}

/// Returns a [`Strategy`] for randomly generating a [`DefineType`] variant.
pub fn define_type() -> impl Strategy<Value = DefineType> {
    prop_oneof![
        Just(DefineType::Public),
        Just(DefineType::Private),
        Just(DefineType::ReadOnly)
    ]
}

/// Returns a [`Strategy`] for randomly generating a [`DataVariableMetadata`]
/// instance.
pub fn data_variable_metadata() -> impl Strategy<Value = DataVariableMetadata> {
    type_signature()
        .prop_map(|value_type| 
            DataVariableMetadata { value_type }
        )

}

/// Returns a [`Strategy`] for randomly generating a [`DataMapMetadata`] instance.
pub fn data_map_metadata() -> impl Strategy<Value = DataMapMetadata> {
    (
        type_signature(),
        type_signature()
    )
    .prop_map(|(key_type, value_type)| 
        DataMapMetadata {
            key_type,
            value_type
        }
    )
}

/// Returns a [`Strategy`] for randomly generating a [`NonFungibleTokenMetadata`] 
/// instance.
pub fn nft_metadata() -> impl Strategy<Value = NonFungibleTokenMetadata> {
    type_signature().prop_map(|key_type| 
        NonFungibleTokenMetadata { key_type }
    )
}

/// Returns a [`Strategy`] for randomly generating a [`FungibleTokenMetadata`] 
/// instance.
pub fn ft_metadata() -> impl Strategy<Value = FungibleTokenMetadata> {
    any::<Option<u128>>().prop_map(|total_supply| 
        FungibleTokenMetadata { total_supply }
    )
}

/// Returns a [`Strategy`] for randomly generating a [`FunctionSignature`] 
/// instance.
pub fn function_signature() -> impl Strategy<Value = FunctionSignature> {
    (
        // arg_types
        prop::collection::vec(type_signature(), 0..3),
        // return_type
        type_signature()
    )
    .prop_map(|(args, returns)| 
        FunctionSignature {
            args,
            returns
        }
    )
}

/// Returns a [`Strategy`] for randomly generating a [`DefinedFunction`]
/// instance.
pub fn defined_function() -> impl Strategy<Value = DefinedFunction> {
    (
        // identifier
        function_identifier(),
        // name
        clarity_name(),
        // arg_types + arguments, which must have the same length
        (0usize..3usize).prop_flat_map(|x| (
            prop::collection::vec(type_signature(), x..=x),
            prop::collection::vec(clarity_name(), x..=x)
        )),
        // define_type
        define_type(),
        // body
        symbolic_expression(),
    )
    .prop_map(|(identifier, name, args, define_type, body)| 
        DefinedFunction {
            identifier,
            name,
            arg_types: args.0,
            define_type,
            arguments: args.1,
            body
        }
    )
}