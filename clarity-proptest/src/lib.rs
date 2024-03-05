use clarity::types::{StacksHashMap, StacksHashSet};
use clarity::vm::contracts::Contract;
use clarity::vm::types::{
    ASCIIData, BuffData, CharType, ListData, ListTypeData, OptionalData, PrincipalData,
    QualifiedContractIdentifier, ResponseData, SequenceData, SequenceSubtype,
    StandardPrincipalData, StringSubtype, StringUTF8Length, TupleData, TupleTypeSignature,
    TypeSignature, UTF8Data, Value, MAX_VALUE_SIZE,
};
use clarity::vm::{ClarityName, ContractContext, ContractName};
use proptest::prelude::*;

pub fn contract() -> impl Strategy<Value = Contract> {
    fn clarity_name() -> impl Strategy<Value = ClarityName> {
        "[a-z]{40}".prop_map(|s| s.try_into().unwrap())
    }
    
    (
        // contract_identifier
        qualified_principal().prop_map(|p| match p {
            Value::Principal(PrincipalData::Contract(qual)) => qual,
            _ => unreachable!(),
        }),
        // variables
        prop::collection::vec((clarity_name(), PropValue::any().prop_map_into()), 0..8)
            .prop_map(|v| StacksHashMap(v.into_iter().collect())),
        // functions
        Just(StacksHashMap::new()),
        // defined_traits
        Just(StacksHashMap::new()),
        // implemented_traits
        Just(StacksHashSet::new()),
        // persisted_names
        Just(StacksHashSet::new()),
        // meta_data_map
        Just(StacksHashMap::new()),
        // meta_data_var
        Just(StacksHashMap::new()),
        // meta_nft
        Just(StacksHashMap::new()),
        // meta_ft
        Just(StacksHashMap::new()),
        // data_size
        0u64..64,
        // clarity_version
        Just(clarity::vm::ClarityVersion::Clarity2),
    )
        .prop_map(
            |(
                contract_identifier,
                variables,
                functions,
                defined_traits,
                implemented_traits,
                persisted_names,
                meta_data_map,
                meta_data_var,
                meta_nft,
                meta_ft,
                data_size,
                clarity_version,
            )| {
                let mut cc = ContractContext::new(contract_identifier, clarity_version);
                cc.variables = variables;
                cc.functions = functions;
                cc.defined_traits = defined_traits;
                cc.implemented_traits = implemented_traits;
                cc.persisted_names = persisted_names;
                cc.meta_data_map = meta_data_map;
                cc.meta_data_var = meta_data_var;
                cc.meta_nft = meta_nft;
                cc.meta_ft = meta_ft;
                cc.data_size = data_size;
                Contract {
                    contract_context: cc,
                }
            },
        )
}

pub fn prop_signature() -> impl Strategy<Value = TypeSignature> {
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
    leaf.prop_recursive(5, 64, 10, |inner| {
        prop_oneof![
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
            (8u32..32, inner.clone()).prop_map(|(s, ty)| (ListTypeData::new_list(ty, s).unwrap()).into()),
        ]
    })
}

#[derive(Clone, PartialEq, Eq)]
pub struct PropValue(Value);

impl From<Value> for PropValue {
    fn from(value: Value) -> Self {
        PropValue(value)
    }
}

impl From<PropValue> for Value {
    fn from(value: PropValue) -> Self {
        value.0
    }
}

impl std::fmt::Debug for PropValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PropValue")
            .field("value", &self.to_string())
            .field("type", &self.type_string())
            .finish()
    }
}

impl std::fmt::Display for PropValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            Value::Sequence(SequenceData::String(clarity::vm::types::CharType::ASCII(
                ASCIIData { data },
            ))) => {
                write!(f, "\"")?;
                for b in data {
                    if [b'\\', b'"'].contains(b) {
                        write!(f, "\\")?;
                    }
                    write!(f, "{}", *b as char)?;
                }
                write!(f, "\"")
            }
            Value::Sequence(SequenceData::String(CharType::UTF8(UTF8Data { data }))) => {
                write!(f, "u\"")?;
                for bytes in data {
                    // SAFETY: a utf8 sequence always contains a valid sequence of utf8 chars as vec of bytes
                    let c = unsafe { std::str::from_utf8_unchecked(bytes).chars().next().unwrap() };
                    match c {
                        '\\' | '\"' => write!(f, "\\{c}")?,
                        _ if c.is_ascii_graphic() => write!(f, "{c}")?,
                        _ => write!(f, r#"\u{{{:X}}}"#, c as u32)?,
                    }
                }
                write!(f, "\"")
            }
            Value::Principal(p) => write!(f, "'{p}"),
            Value::Optional(OptionalData { data }) => match data {
                Some(inner) => write!(f, "(some {})", PropValue(*inner.clone())),
                None => write!(f, "none"),
            },
            Value::Response(ResponseData { committed, data }) => {
                if *committed {
                    write!(f, "(ok {})", PropValue(*data.clone()))
                } else {
                    write!(f, "(err {})", PropValue(*data.clone()))
                }
            }
            Value::Sequence(SequenceData::List(ListData { data, .. })) => {
                write!(f, "(list")?;
                for d in data {
                    write!(f, " ")?;
                    write!(f, "{}", PropValue(d.clone()))?;
                }
                write!(f, ")")
            }
            Value::Tuple(data) => {
                write!(f, "(tuple")?;
                for (key, value) in &data.data_map {
                    write!(f, " ")?;
                    write!(f, "({} {})", &**key, PropValue(value.clone()))?;
                }
                write!(f, ")")
            }
            otherwise => write!(f, "{otherwise}"),
        }
    }
}

impl PropValue {
    pub fn any() -> impl Strategy<Value = Self> {
        prop_signature().prop_flat_map(prop_value).prop_map_into()
    }

    pub fn from_type(ty: TypeSignature) -> impl Strategy<Value = Self> {
        prop_value(ty).prop_map_into()
    }

    pub fn many_from_type(ty: TypeSignature, count: usize) -> impl Strategy<Value = Vec<Self>> {
        prop::collection::vec(Self::from_type(ty.clone()), count)
    }

    pub fn any_sequence(size: usize) -> impl Strategy<Value = Self> {
        let any_list = prop_signature()
            .prop_ind_flat_map2(move |ty| prop::collection::vec(prop_value(ty), size))
            .prop_map(move |(ty, vec)| {
                Value::Sequence(SequenceData::List(ListData {
                    data: vec,
                    type_signature: ListTypeData::new_list(ty, size as u32).unwrap(),
                }))
            });
        // TODO: add string-utf8
        prop_oneof![
            // 10% chance for a buffer
            1 => buffer(size as u32),
            // 10% chance for a string-ascii
            1 => string_ascii(size as u32),
            // 80% chance for a list
            8 => any_list
        ]
        .prop_map_into()
    }
}

impl TryFrom<Vec<PropValue>> for PropValue {
    type Error = clarity::vm::errors::Error;

    fn try_from(values: Vec<PropValue>) -> Result<Self, Self::Error> {
        let values = values.into_iter().map(Value::from).collect();
        Value::cons_list_unsanitized(values).map(PropValue::from)
    }
}

fn prop_value(ty: TypeSignature) -> impl Strategy<Value = Value> {
    match ty {
        TypeSignature::NoType => unreachable!(),
        TypeSignature::IntType => int().boxed(),
        TypeSignature::UIntType => uint().boxed(),
        TypeSignature::BoolType => bool().boxed(),
        TypeSignature::OptionalType(ty) => optional(*ty).boxed(),
        TypeSignature::ResponseType(ok_err) => response(ok_err.0, ok_err.1).boxed(),
        TypeSignature::SequenceType(SequenceSubtype::BufferType(size)) => {
            buffer(size.into()).boxed()
        }
        TypeSignature::SequenceType(SequenceSubtype::StringType(StringSubtype::ASCII(size))) => {
            string_ascii(size.into()).boxed()
        }
        TypeSignature::SequenceType(SequenceSubtype::StringType(StringSubtype::UTF8(size))) => {
            string_utf8(size.into()).boxed()
        }
        TypeSignature::SequenceType(SequenceSubtype::ListType(list_type_data)) => {
            list(list_type_data).boxed()
        }
        TypeSignature::TupleType(tuple_ty) => tuple(tuple_ty).boxed(),
        TypeSignature::PrincipalType => {
            prop_oneof![standard_principal(), qualified_principal()].boxed()
        }
        // TODO
        TypeSignature::ListUnionType(_) => todo!(),
        TypeSignature::CallableType(_) => todo!(),
        TypeSignature::TraitReferenceType(_) => todo!(),
    }
}

fn int() -> impl Strategy<Value = Value> {
    any::<i128>().prop_map(Value::Int)
}

fn uint() -> impl Strategy<Value = Value> {
    any::<u128>().prop_map(Value::UInt)
}

fn bool() -> impl Strategy<Value = Value> {
    any::<bool>().prop_map(Value::Bool)
}

pub fn string_ascii(size: u32) -> impl Strategy<Value = Value> {
    let size = size as usize;
    prop::collection::vec(0x20u8..0x7e, size..=size).prop_map(|bytes| {
        Value::Sequence(SequenceData::String(clarity::vm::types::CharType::ASCII(
            clarity::vm::types::ASCIIData { data: bytes },
        )))
    })
}

pub fn string_utf8(size: u32) -> impl Strategy<Value = Value> {
    prop::collection::vec(any::<char>(), size as usize).prop_map(|chars| {
        let mut data = Vec::with_capacity(chars.len());
        for c in chars {
            let mut encoded_char = vec![0; c.len_utf8()];
            c.encode_utf8(encoded_char.as_mut());
            data.push(encoded_char);
        }
        Value::Sequence(SequenceData::String(CharType::UTF8(UTF8Data { data })))
    })
}

fn buffer(size: u32) -> impl Strategy<Value = Value> {
    let size = size as usize;
    prop::collection::vec(any::<u8>(), size..=size)
        .prop_map(|bytes| Value::Sequence(SequenceData::Buffer(BuffData { data: bytes })))
}

fn optional(inner_ty: TypeSignature) -> impl Strategy<Value = Value> {
    match inner_ty {
        TypeSignature::NoType => Just(Value::none()).boxed(),
        _ => prop::option::of(prop_value(inner_ty))
            .prop_map(|v| {
                Value::Optional(OptionalData {
                    data: v.map(Box::new),
                })
            })
            .boxed(),
    }
}

fn response(ok_ty: TypeSignature, err_ty: TypeSignature) -> impl Strategy<Value = Value> {
    match (ok_ty, err_ty) {
        (TypeSignature::NoType, err_ty) => prop_value(err_ty)
            .prop_map(|err| {
                Value::Response(ResponseData {
                    committed: false,
                    data: Box::new(err),
                })
            })
            .boxed(),
        (ok_ty, TypeSignature::NoType) => prop_value(ok_ty)
            .prop_map(|ok| {
                Value::Response(ResponseData {
                    committed: true,
                    data: Box::new(ok),
                })
            })
            .boxed(),
        (ok_ty, err_ty) => prop::result::maybe_err(prop_value(ok_ty), prop_value(err_ty))
            .prop_map(|res| {
                Value::Response(ResponseData {
                    committed: res.is_ok(),
                    data: res.map_or_else(Box::new, Box::new),
                })
            })
            .boxed(),
    }
}

fn list(list_type_data: ListTypeData) -> impl Strategy<Value = Value> {
    prop::collection::vec(
        prop_value(list_type_data.get_list_item_type().clone()),
        0..=list_type_data.get_max_len() as usize,
    )
    .prop_map(move |v| {
        Value::Sequence(SequenceData::List(ListData {
            data: v,
            type_signature: list_type_data.clone(),
        }))
    })
}

fn tuple(tuple_ty: TupleTypeSignature) -> impl Strategy<Value = Value> {
    let fields: Vec<_> = tuple_ty.get_type_map().keys().cloned().collect();
    let strategies: Vec<_> = tuple_ty
        .get_type_map()
        .values()
        .cloned()
        .map(prop_value)
        .collect();
    strategies.prop_map(move |vec_values| {
        TupleData {
            type_signature: tuple_ty.clone(),
            data_map: fields.clone().into_iter().zip(vec_values).collect(),
        }
        .into()
    })
}

fn standard_principal() -> impl Strategy<Value = Value> {
    (0u8..32, prop::collection::vec(any::<u8>(), 20))
        .prop_map(|(v, hash)| {
            Value::Principal(PrincipalData::Standard(StandardPrincipalData(
                v,
                hash.try_into().unwrap(),
            )))
        })
        .no_shrink()
}

fn qualified_principal() -> impl Strategy<Value = Value> {
    (standard_principal(), "[a-zA-Z]{1,40}").prop_map(|(issuer_value, name)| {
        let Value::Principal(PrincipalData::Standard(issuer)) = issuer_value else {
            unreachable!()
        };
        let name = ContractName::from(&*name);
        Value::Principal(PrincipalData::Contract(QualifiedContractIdentifier {
            issuer,
            name,
        }))
    })
}

trait TypePrinter {
    fn type_string(&self) -> String;
}

impl TypePrinter for PropValue {
    fn type_string(&self) -> String {
        self.0.type_string()
    }
}

impl TypePrinter for Value {
    fn type_string(&self) -> String {
        match &self {
            Value::Int(_) => type_string(&TypeSignature::IntType),
            Value::UInt(_) => type_string(&TypeSignature::UIntType),
            Value::Bool(_) => type_string(&TypeSignature::BoolType),
            Value::Sequence(SequenceData::Buffer(length)) => {
                type_string(&TypeSignature::SequenceType(SequenceSubtype::BufferType(
                    length
                        .len()
                        .expect("Failed to get buffer length from sequence data"),
                )))
            }
            Value::Sequence(SequenceData::String(CharType::ASCII(data))) => type_string(
                &TypeSignature::SequenceType(SequenceSubtype::StringType(StringSubtype::ASCII(
                    data.len()
                        .expect("Failed to get ASCII string length from sequence data"),
                ))),
            ),
            Value::Sequence(SequenceData::String(CharType::UTF8(data))) => type_string(
                &TypeSignature::SequenceType(SequenceSubtype::StringType(StringSubtype::UTF8(
                    StringUTF8Length::try_from(u32::from(
                        data.len()
                            .expect("Failed to get UTF8 string length from sequence data"),
                    ))
                    .unwrap_or(StringUTF8Length::try_from(MAX_VALUE_SIZE / 4).unwrap()),
                ))),
            ),
            Value::Optional(inner) => inner.type_string(),
            Value::Response(inner) => inner.type_string(),
            Value::Sequence(SequenceData::List(list_data)) => list_data.type_string(),
            Value::Tuple(data) => data.type_string(),
            Value::Principal(_) => type_string(&TypeSignature::PrincipalType),
            Value::CallableContract(_) => type_string(&TypeSignature::PrincipalType),
        }
    }
}

impl TypePrinter for OptionalData {
    fn type_string(&self) -> String {
        let inner = match self.data {
            Some(ref inner) => inner.type_string(),
            None => "int".to_owned(), // We need to default to something here
        };
        format!("(optional {inner})")
    }
}

impl TypePrinter for ResponseData {
    fn type_string(&self) -> String {
        let (ok_string, err_string) = if self.committed {
            (self.data.type_string(), "int".to_owned())
        } else {
            ("int".to_owned(), self.data.type_string())
        };
        format!("(response {} {})", ok_string, err_string)
    }
}

impl TypePrinter for ListData {
    fn type_string(&self) -> String {
        format!(
            "(list {} {})",
            self.data.len(),
            type_string(self.type_signature.get_list_item_type())
        )
    }
}

impl TypePrinter for TupleData {
    fn type_string(&self) -> String {
        type_string(&TypeSignature::TupleType(self.type_signature.clone()))
    }
}

pub fn type_string(ty: &TypeSignature) -> String {
    match ty {
        TypeSignature::IntType => "int".to_owned(),
        TypeSignature::UIntType => "uint".to_owned(),
        TypeSignature::BoolType => "bool".to_owned(),
        TypeSignature::OptionalType(inner) => format!("(optional {})", type_string(inner)),
        TypeSignature::ResponseType(inner) => format!(
            "(response {} {})",
            type_string(&inner.0),
            type_string(&inner.1)
        ),
        TypeSignature::SequenceType(SequenceSubtype::BufferType(len)) => format!("(buff {len})"),
        TypeSignature::SequenceType(SequenceSubtype::StringType(StringSubtype::ASCII(len))) => {
            format!("(string-ascii {len})")
        }
        TypeSignature::SequenceType(SequenceSubtype::StringType(StringSubtype::UTF8(len))) => {
            format!("(string-utf8 {len})")
        }
        TypeSignature::SequenceType(SequenceSubtype::ListType(list_type_data)) => {
            format!(
                "(list {} {})",
                list_type_data.get_max_len(),
                type_string(list_type_data.get_list_item_type())
            )
        }
        TypeSignature::TupleType(tuple_ty) => {
            let mut s = String::new();
            s.push('{');
            for (key, value) in tuple_ty.get_type_map() {
                s.push_str(key);
                s.push(':');
                s.push_str(&type_string(value));
                s.push(',');
            }
            s.push('}');
            s
        }
        TypeSignature::PrincipalType => "principal".to_owned(),
        TypeSignature::CallableType(_) => "principal".to_owned(),
        TypeSignature::TraitReferenceType(_) => "principal".to_owned(),
        TypeSignature::NoType => "int".to_owned(), // Use "int" as a default type
        TypeSignature::ListUnionType(_) => unreachable!(),
    }
}
