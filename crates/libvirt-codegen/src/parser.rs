//! Parser for XDR protocol definition files (.x files).

use crate::ast::*;
use nom::{
    branch::alt,
    bytes::complete::{tag, take_while, take_while1},
    character::complete::{char, digit1, multispace0, multispace1},
    combinator::{all_consuming, map, map_res, opt, recognize, value},
    multi::{many0, separated_list0},
    sequence::{delimited, pair, preceded, terminated},
    IResult,
};
use std::path::Path;

/// Parse a protocol definition file.
pub fn parse_file(path: impl AsRef<Path>) -> Result<Protocol, String> {
    let content =
        std::fs::read_to_string(path.as_ref()).map_err(|e| format!("failed to read file: {}", e))?;
    parse_protocol(&content)
}

/// Parse protocol definition from string.
pub fn parse_protocol(input: &str) -> Result<Protocol, String> {
    // Preprocess: remove comments
    let input = remove_comments(input);

    let result = all_consuming(protocol_parser)(&input);
    match result {
        Ok((_, protocol)) => Ok(protocol),
        Err(e) => Err(format!("parse error: {:?}", e)),
    }
}

/// Remove C-style comments, preprocessor directives, and XDR passthrough lines.
fn remove_comments(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    let mut at_line_start = true;

    while let Some(c) = chars.next() {
        if c == '/' {
            match chars.peek() {
                Some('*') => {
                    // Block comment
                    chars.next();
                    while let Some(c) = chars.next() {
                        if c == '*' && chars.peek() == Some(&'/') {
                            chars.next();
                            result.push(' '); // Replace with space to preserve line structure
                            break;
                        }
                    }
                    at_line_start = false;
                }
                Some('/') => {
                    // Line comment
                    chars.next();
                    while let Some(&c) = chars.peek() {
                        if c == '\n' {
                            break;
                        }
                        chars.next();
                    }
                }
                _ => {
                    result.push(c);
                    at_line_start = false;
                }
            }
        } else if c == '#' || (c == '%' && at_line_start) {
            // Preprocessor directive or XDR passthrough - skip entire line
            while let Some(&c) = chars.peek() {
                if c == '\n' {
                    result.push('\n');
                    at_line_start = true;
                    break;
                }
                chars.next();
            }
        } else if c == '\n' {
            result.push(c);
            at_line_start = true;
        } else if c.is_whitespace() {
            result.push(c);
            // Don't change at_line_start for spaces
        } else {
            result.push(c);
            at_line_start = false;
        }
    }

    result
}

/// Resolve well-known libvirt constants to their values.
fn resolve_well_known_constant(name: &str) -> Option<u32> {
    match name {
        "VIR_UUID_BUFLEN" => Some(16),
        "VIR_UUID_STRING_BUFLEN" => Some(37),
        _ => None,
    }
}

// Helper parsers

fn ws<'a, F, O, E>(inner: F) -> impl FnMut(&'a str) -> IResult<&'a str, O, E>
where
    F: FnMut(&'a str) -> IResult<&'a str, O, E>,
    E: nom::error::ParseError<&'a str>,
{
    delimited(multispace0, inner, multispace0)
}

fn identifier(input: &str) -> IResult<&str, &str> {
    recognize(pair(
        take_while1(|c: char| c.is_ascii_alphabetic() || c == '_'),
        take_while(|c: char| c.is_ascii_alphanumeric() || c == '_'),
    ))(input)
}

fn integer(input: &str) -> IResult<&str, i64> {
    alt((
        // Hex number
        map_res(
            preceded(
                alt((tag("0x"), tag("0X"))),
                take_while1(|c: char| c.is_ascii_hexdigit()),
            ),
            |s: &str| i64::from_str_radix(s, 16),
        ),
        // Decimal (possibly negative)
        map_res(recognize(pair(opt(char('-')), digit1)), |s: &str| {
            s.parse::<i64>()
        }),
    ))(input)
}

fn const_value(input: &str) -> IResult<&str, ConstValue> {
    alt((
        map(integer, ConstValue::Int),
        map(identifier, |s| ConstValue::Ident(s.to_string())),
    ))(input)
}

// Protocol parser

fn protocol_parser(input: &str) -> IResult<&str, Protocol> {
    let (input, items) = many0(ws(definition))(input)?;

    let mut protocol = Protocol::new("remote");

    for item in items {
        match item {
            Definition::Const(c) => protocol.constants.push(c),
            Definition::Type(t) => protocol.types.push(t),
        }
    }

    // Extract program ID and protocol version from constants
    extract_protocol_metadata(&mut protocol);

    // Extract procedures from procedure enum
    extract_procedures(&mut protocol);

    Ok((input, protocol))
}

/// Extract program ID and protocol version from constants.
fn extract_protocol_metadata(protocol: &mut Protocol) {
    for constant in &protocol.constants {
        match constant.name.as_str() {
            "REMOTE_PROGRAM" | "QEMU_PROGRAM" | "LXC_PROGRAM" => {
                if let ConstValue::Int(v) = &constant.value {
                    protocol.program_id = Some(*v as u32);
                }
                // Set protocol name based on program constant
                if constant.name.starts_with("QEMU") {
                    protocol.name = "qemu".to_string();
                    protocol.proc_prefix = Some("QEMU_PROC".to_string());
                } else if constant.name.starts_with("LXC") {
                    protocol.name = "lxc".to_string();
                    protocol.proc_prefix = Some("LXC_PROC".to_string());
                } else {
                    protocol.name = "remote".to_string();
                    protocol.proc_prefix = Some("REMOTE_PROC".to_string());
                }
            }
            "REMOTE_PROTOCOL_VERSION" | "QEMU_PROTOCOL_VERSION" | "LXC_PROTOCOL_VERSION" => {
                if let ConstValue::Int(v) = &constant.value {
                    protocol.protocol_version = Some(*v as u32);
                }
            }
            _ => {}
        }
    }
}

/// Extract procedure definitions from the procedure enum.
///
/// Each procedure like REMOTE_PROC_DOMAIN_LOOKUP_BY_NAME = 23 maps to:
/// - args type: remote_domain_lookup_by_name_args (if exists)
/// - ret type: remote_domain_lookup_by_name_ret (if exists)
fn extract_procedures(protocol: &mut Protocol) {
    // Determine the procedure enum name and prefix based on protocol type
    let (enum_name, proc_prefix, type_prefix) = match protocol.name.as_str() {
        "qemu" => ("qemu_procedure", "QEMU_PROC_", "qemu_"),
        "lxc" => ("lxc_procedure", "LXC_PROC_", "lxc_"),
        _ => ("remote_procedure", "REMOTE_PROC_", "remote_"),
    };

    // Find the procedure enum
    let procedure_enum = protocol
        .types
        .iter()
        .find_map(|t| {
            if let TypeDef::Enum(e) = t {
                if e.name == enum_name {
                    return Some(e.clone());
                }
            }
            None
        });

    let procedure_enum = match procedure_enum {
        Some(e) => e,
        None => return,
    };

    // Collect all struct names for lookup
    let struct_names: std::collections::HashSet<String> = protocol
        .types
        .iter()
        .filter_map(|t| {
            if let TypeDef::Struct(s) = t {
                Some(s.name.clone())
            } else {
                None
            }
        })
        .collect();

    // Convert each enum variant to a procedure
    for variant in &procedure_enum.variants {
        let number = match &variant.value {
            Some(ConstValue::Int(n)) => *n as u32,
            _ => continue,
        };

        // Convert REMOTE_PROC_DOMAIN_LOOKUP_BY_NAME to domain_lookup_by_name
        let base_name = variant
            .name
            .strip_prefix(proc_prefix)
            .unwrap_or(&variant.name)
            .to_lowercase();

        let args_name = format!("{}{}_args", type_prefix, base_name);
        let ret_name = format!("{}{}_ret", type_prefix, base_name);

        let args = if struct_names.contains(&args_name) {
            Some(args_name)
        } else {
            None
        };

        let ret = if struct_names.contains(&ret_name) {
            Some(ret_name)
        } else {
            None
        };

        protocol.procedures.push(Procedure {
            name: variant.name.clone(),
            number,
            args,
            ret,
            priority: Priority::default(),
        });
    }
}

enum Definition {
    Const(Constant),
    Type(TypeDef),
}

fn definition(input: &str) -> IResult<&str, Definition> {
    alt((
        map(const_def, Definition::Const),
        map(type_def, Definition::Type),
    ))(input)
}

// Constant definition: const NAME = VALUE;
fn const_def(input: &str) -> IResult<&str, Constant> {
    let (input, _) = tag("const")(input)?;
    let (input, _) = multispace1(input)?;
    let (input, name) = identifier(input)?;
    let (input, _) = ws(char('='))(input)?;
    let (input, value) = const_value(input)?;
    let (input, _) = ws(char(';'))(input)?;

    Ok((
        input,
        Constant {
            name: name.to_string(),
            value,
        },
    ))
}

// Type definitions
fn type_def(input: &str) -> IResult<&str, TypeDef> {
    alt((
        map(struct_def, TypeDef::Struct),
        map(enum_def, TypeDef::Enum),
        map(union_def, TypeDef::Union),
        map(typedef_def, TypeDef::Typedef),
    ))(input)
}

// Struct definition: struct NAME { fields };
fn struct_def(input: &str) -> IResult<&str, StructDef> {
    let (input, _) = tag("struct")(input)?;
    let (input, _) = multispace1(input)?;
    let (input, name) = identifier(input)?;
    let (input, _) = ws(char('{'))(input)?;
    let (input, fields) = many0(ws(field_def))(input)?;
    let (input, _) = ws(char('}'))(input)?;
    let (input, _) = ws(char(';'))(input)?;

    Ok((
        input,
        StructDef {
            name: name.to_string(),
            fields,
        },
    ))
}

// Field definition: TYPE NAME;
fn field_def(input: &str) -> IResult<&str, Field> {
    let (input, ty) = type_spec(input)?;
    let (input, _) = multispace1(input)?;
    let (input, name) = identifier(input)?;
    // Handle array suffix
    let (input, ty) = array_suffix(input, ty)?;
    let (input, _) = ws(char(';'))(input)?;

    Ok((
        input,
        Field {
            name: name.to_string(),
            ty,
        },
    ))
}

// Handle array suffix [N] or <N>
fn array_suffix(input: &str, base_ty: Type) -> IResult<&str, Type> {
    let input_trimmed = input.trim_start();

    if input_trimmed.starts_with('[') {
        // Fixed array [N]
        let (input, _) = multispace0(input)?;
        let (input, _) = char('[')(input)?;
        let (input, len) = ws(const_value)(input)?;
        let (input, _) = char(']')(input)?;

        let size = match &len {
            ConstValue::Int(n) => *n as u32,
            ConstValue::Ident(name) => resolve_well_known_constant(name).unwrap_or(0),
        };

        // For opaque[N], return fixed-length opaque instead of array
        match &base_ty {
            Type::Opaque { .. } => Ok((
                input,
                Type::Opaque {
                    len: LengthSpec::Fixed(size),
                },
            )),
            _ => Ok((
                input,
                Type::Array {
                    elem: Box::new(base_ty),
                    len: LengthSpec::Fixed(size),
                },
            )),
        }
    } else if input_trimmed.starts_with('<') {
        // Variable array <N> or <>
        // BUT: string<N> is just a string with max length, not an array!
        // Same for opaque<N>
        match &base_ty {
            Type::String { .. } | Type::Opaque { .. } => {
                // For string and opaque, <N> just sets max length, type stays the same
                let (input, _) = multispace0(input)?;
                let (input, _) = char('<')(input)?;
                let (input, len) = ws(opt(const_value))(input)?;
                let (input, _) = char('>')(input)?;

                let max = len.and_then(|v| match v {
                    ConstValue::Int(n) => Some(n as u32),
                    ConstValue::Ident(_) => None,
                });

                // Return the same type, possibly with updated max length
                match base_ty {
                    Type::String { .. } => Ok((input, Type::String { max_len: max })),
                    Type::Opaque { .. } => Ok((
                        input,
                        Type::Opaque {
                            len: LengthSpec::Variable { max },
                        },
                    )),
                    _ => unreachable!(),
                }
            }
            _ => {
                // For other types, <N> means variable-length array
                let (input, _) = multispace0(input)?;
                let (input, _) = char('<')(input)?;
                let (input, len) = ws(opt(const_value))(input)?;
                let (input, _) = char('>')(input)?;

                let max = len.and_then(|v| match v {
                    ConstValue::Int(n) => Some(n as u32),
                    ConstValue::Ident(_) => None,
                });

                Ok((
                    input,
                    Type::Array {
                        elem: Box::new(base_ty),
                        len: LengthSpec::Variable { max },
                    },
                ))
            }
        }
    } else {
        // No suffix
        Ok((input, base_ty))
    }
}

// Type specification
fn type_spec(input: &str) -> IResult<&str, Type> {
    alt((
        value(Type::Void, tag("void")),
        // unsigned types
        value(
            Type::UHyper,
            pair(tag("unsigned"), preceded(multispace1, tag("hyper"))),
        ),
        value(
            Type::UInt,
            pair(tag("unsigned"), preceded(multispace1, tag("int"))),
        ),
        // unsigned char -> u8
        map(
            pair(tag("unsigned"), preceded(multispace1, tag("char"))),
            |_| Type::Named("u8".to_string()),
        ),
        // unsigned short -> u16
        map(
            pair(tag("unsigned"), preceded(multispace1, tag("short"))),
            |_| Type::Named("u16".to_string()),
        ),
        // char -> i8
        value(Type::Named("i8".to_string()), tag("char")),
        // short -> i16
        value(Type::Named("i16".to_string()), tag("short")),
        value(Type::Hyper, tag("hyper")),
        value(Type::Int, tag("int")),
        value(Type::Float, tag("float")),
        value(Type::Double, tag("double")),
        value(Type::Bool, tag("bool")),
        string_type,
        opaque_type,
        optional_type,
        map(identifier, |s| Type::Named(s.to_string())),
    ))(input)
}

// Optional type: TYPE *
fn optional_type(input: &str) -> IResult<&str, Type> {
    let (input, ty) = alt((
        value(
            Type::UHyper,
            pair(tag("unsigned"), preceded(multispace1, tag("hyper"))),
        ),
        value(
            Type::UInt,
            pair(tag("unsigned"), preceded(multispace1, tag("int"))),
        ),
        value(Type::Hyper, tag("hyper")),
        value(Type::Int, tag("int")),
        value(Type::Float, tag("float")),
        value(Type::Double, tag("double")),
        value(Type::Bool, tag("bool")),
        map(identifier, |s| Type::Named(s.to_string())),
    ))(input)?;

    let (input, _) = ws(char('*'))(input)?;

    Ok((input, Type::Optional(Box::new(ty))))
}

// String type: string<N> or string<>
fn string_type(input: &str) -> IResult<&str, Type> {
    let (input, _) = tag("string")(input)?;
    let (input, max_len) = opt(delimited(char('<'), ws(opt(integer)), char('>')))(input)?;

    let max_len = max_len.flatten().map(|n| n as u32);

    Ok((input, Type::String { max_len }))
}

// Opaque type: opaque NAME[N] or opaque NAME<N>
fn opaque_type(input: &str) -> IResult<&str, Type> {
    let (input, _) = tag("opaque")(input)?;

    Ok((
        input,
        Type::Opaque {
            len: LengthSpec::Variable { max: None },
        },
    ))
}

// Enum definition: enum NAME { variants };
fn enum_def(input: &str) -> IResult<&str, EnumDef> {
    let (input, _) = tag("enum")(input)?;
    let (input, _) = multispace1(input)?;
    let (input, name) = identifier(input)?;
    let (input, _) = ws(char('{'))(input)?;
    let (input, variants) = separated_list0(ws(char(',')), ws(enum_variant))(input)?;
    let (input, _) = opt(ws(char(',')))(input)?; // trailing comma
    let (input, _) = ws(char('}'))(input)?;
    let (input, _) = ws(char(';'))(input)?;

    Ok((
        input,
        EnumDef {
            name: name.to_string(),
            variants,
        },
    ))
}

// Enum variant: NAME = VALUE or NAME
fn enum_variant(input: &str) -> IResult<&str, EnumVariant> {
    let (input, name) = identifier(input)?;
    let (input, value) = opt(preceded(ws(char('=')), const_value))(input)?;

    Ok((
        input,
        EnumVariant {
            name: name.to_string(),
            value,
        },
    ))
}

// Union definition
fn union_def(input: &str) -> IResult<&str, UnionDef> {
    let (input, _) = tag("union")(input)?;
    let (input, _) = multispace1(input)?;
    let (input, name) = identifier(input)?;
    let (input, _) = ws(tag("switch"))(input)?;
    let (input, _) = ws(char('('))(input)?;
    let (input, disc_ty) = type_spec(input)?;
    let (input, _) = multispace1(input)?;
    let (input, disc_name) = identifier(input)?;
    let (input, _) = ws(char(')'))(input)?;
    let (input, _) = ws(char('{'))(input)?;
    let (input, cases) = many0(ws(union_case))(input)?;
    let (input, default) = opt(union_default)(input)?;
    let (input, _) = ws(char('}'))(input)?;
    let (input, _) = ws(char(';'))(input)?;

    Ok((
        input,
        UnionDef {
            name: name.to_string(),
            discriminant: Field {
                name: disc_name.to_string(),
                ty: disc_ty,
            },
            cases,
            default,
        },
    ))
}

// Union case: case VALUE: FIELD;
fn union_case(input: &str) -> IResult<&str, UnionCase> {
    let (input, _) = tag("case")(input)?;
    let (input, _) = multispace1(input)?;
    let (input, value) = const_value(input)?;
    let (input, _) = ws(char(':'))(input)?;

    // Field or void
    let (input, field) = alt((
        map(field_def, Some),
        map(terminated(tag("void"), ws(char(';'))), |_| None),
    ))(input)?;

    Ok((
        input,
        UnionCase {
            values: vec![value],
            field,
        },
    ))
}

// Union default: default: FIELD;
fn union_default(input: &str) -> IResult<&str, Box<Type>> {
    let (input, _) = ws(tag("default"))(input)?;
    let (input, _) = ws(char(':'))(input)?;
    let (input, field) = field_def(input)?;

    Ok((input, Box::new(field.ty)))
}

// Typedef: typedef TYPE NAME; or typedef TYPE *NAME;
fn typedef_def(input: &str) -> IResult<&str, TypedefDef> {
    let (input, _) = tag("typedef")(input)?;
    let (input, _) = multispace1(input)?;
    let (input, target) = type_spec(input)?;
    let (input, _) = multispace0(input)?;

    // Check for pointer typedef: typedef TYPE *NAME;
    let (input, is_pointer) = opt(char('*'))(input)?;
    let (input, _) = multispace0(input)?;
    let (input, name) = identifier(input)?;

    // Handle array suffix
    let (input, target) = array_suffix(input, target)?;
    let (input, _) = ws(char(';'))(input)?;

    let target = if is_pointer.is_some() {
        Type::Optional(Box::new(target))
    } else {
        target
    };

    Ok((
        input,
        TypedefDef {
            name: name.to_string(),
            target,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove_comments() {
        let input = r#"
            /* block comment */
            const FOO = 1; // line comment
            # preprocessor
            const BAR = 2;
        "#;
        let result = remove_comments(input);
        assert!(!result.contains("block comment"));
        assert!(!result.contains("line comment"));
        assert!(!result.contains("preprocessor"));
    }

    #[test]
    fn test_parse_const() {
        let input = "const FOO = 42;";
        let (_, c) = const_def(input).unwrap();
        assert_eq!(c.name, "FOO");
        assert!(matches!(c.value, ConstValue::Int(42)));
    }

    #[test]
    fn test_parse_struct() {
        let input = r#"
            struct Point {
                int x;
                int y;
            };
        "#;
        let result = parse_protocol(input).unwrap();
        assert_eq!(result.types.len(), 1);

        if let TypeDef::Struct(s) = &result.types[0] {
            assert_eq!(s.name, "Point");
            assert_eq!(s.fields.len(), 2);
        } else {
            panic!("expected struct");
        }
    }

    #[test]
    fn test_parse_enum() {
        let input = r#"
            enum Color {
                RED = 0,
                GREEN = 1,
                BLUE = 2
            };
        "#;
        let result = parse_protocol(input).unwrap();
        assert_eq!(result.types.len(), 1);

        if let TypeDef::Enum(e) = &result.types[0] {
            assert_eq!(e.name, "Color");
            assert_eq!(e.variants.len(), 3);
        } else {
            panic!("expected enum");
        }
    }

    #[test]
    fn test_parse_typedef() {
        let input = "typedef string remote_string<>;";
        let result = parse_protocol(input).unwrap();
        assert_eq!(result.types.len(), 1);

        if let TypeDef::Typedef(t) = &result.types[0] {
            assert_eq!(t.name, "remote_string");
        } else {
            panic!("expected typedef");
        }
    }
}
