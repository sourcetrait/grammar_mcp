//! The contract's typedefs, read from a signature, and values held to them.
use crate::*;

/// A scalar a typedef names, one of nushell's own.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ser::Serialize, ser::Deserialize)]
pub enum NuScalarTypedef {
    Int,
    Float,
    String,
    Bool,
    Datetime,
    Duration,
    Filesize,
    Binary,
    Range,
    Number,
    Glob,
    CellPath,
    Path,
    Directory,
}

impl NuScalarTypedef {
    /// The scalar's name as nushell spells it.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Int => "int",
            Self::Float => "float",
            Self::String => "string",
            Self::Bool => "bool",
            Self::Datetime => "datetime",
            Self::Duration => "duration",
            Self::Filesize => "filesize",
            Self::Binary => "binary",
            Self::Range => "range",
            Self::Number => "number",
            Self::Glob => "glob",
            Self::CellPath => "cell-path",
            Self::Path => "path",
            Self::Directory => "directory",
        }
    }
}

/// A record field's name.
#[derive(Clone, Debug, PartialEq, Eq, ser::Serialize, ser::Deserialize)]
pub struct FieldName(pub String);

/// A table column's name.
#[derive(Clone, Debug, PartialEq, Eq, ser::Serialize, ser::Deserialize)]
pub struct ColumnName(pub String);

/// A type the contract holds a value to: a scalar, nothing, or a collection.
#[derive(Clone, Debug, PartialEq, Eq, ser::Serialize, ser::Deserialize)]
pub enum NuTypedef {
    Scalar(NuScalarTypedef),
    Nothing,
    Record(NuRecordTypedef),
    Oneof(NuOneofTypedef),
    Table(NuTableTypedef),
    List(NuListTypedef),
}

/// A record's declared fields; none declared is an open record.
#[derive(Clone, Debug, PartialEq, Eq, ser::Serialize, ser::Deserialize)]
pub struct NuRecordTypedef {
    pub fields: Vec<NuRecordFieldTypedef>,
}

/// One declared field and its type.
#[derive(Clone, Debug, PartialEq, Eq, ser::Serialize, ser::Deserialize)]
pub struct NuRecordFieldTypedef {
    pub name: FieldName,
    pub typedef: NuTypedef,
}

/// A table's declared columns; none declared is an open table.
#[derive(Clone, Debug, PartialEq, Eq, ser::Serialize, ser::Deserialize)]
pub struct NuTableTypedef {
    pub columns: Vec<NuTableColumnTypedef>,
}

/// One declared column and its type.
#[derive(Clone, Debug, PartialEq, Eq, ser::Serialize, ser::Deserialize)]
pub struct NuTableColumnTypedef {
    pub name: ColumnName,
    pub typedef: NuTypedef,
}

/// A union's member types.
#[derive(Clone, Debug, PartialEq, Eq, ser::Serialize, ser::Deserialize)]
pub struct NuOneofTypedef {
    pub members: Vec<NuTypedef>,
}

/// A list's element type.
#[derive(Clone, Debug, PartialEq, Eq, ser::Serialize, ser::Deserialize)]
pub struct NuListTypedef {
    pub element: Box<NuTypedef>,
}

/// A typedef as nushell spells it.
fn render_nu_typedef(t: &NuTypedef) -> std::string::String {
    match t {
        NuTypedef::Scalar(s) => s.name().to_string(),
        NuTypedef::Nothing => "nothing".to_string(),
        NuTypedef::Record(r) => render_nu_record(&r.fields),
        NuTypedef::Oneof(o) => {
            let parts: Vec<std::string::String> = o.members.iter().map(render_nu_typedef).collect();
            format!("oneof<{}>", parts.join(", "))
        }
        NuTypedef::Table(tab) => {
            let cols: Vec<std::string::String> = tab
                .columns
                .iter()
                .map(|c| format!("{}: {}", c.name.0, render_nu_typedef(&c.typedef)))
                .collect();
            format!("table<{}>", cols.join(", "))
        }
        NuTypedef::List(l) => {
            format!("list<{}>", render_nu_typedef(&l.element))
        }
    }
}

/// A record's fields as nushell spells them.
fn render_nu_record(fields: &[NuRecordFieldTypedef]) -> std::string::String {
    let parts: Vec<std::string::String> = fields
        .iter()
        .map(|f| format!("{}: {}", f.name.0, render_nu_typedef(&f.typedef)))
        .collect();
    format!("record<{}>", parts.join(", "))
}

/// A typedef in the signature block's compact spelling, no spaces.
fn render_signature_typedef(t: &NuTypedef) -> std::string::String {
    match t {
        NuTypedef::Scalar(s) => s.name().to_string(),
        NuTypedef::Nothing => "nothing".to_string(),
        NuTypedef::Record(r) => format!("record<{}>", render_signature_fields(&r.fields)),
        NuTypedef::Oneof(o) => {
            let parts: Vec<std::string::String> =
                o.members.iter().map(render_signature_typedef).collect();
            format!("oneof<{}>", parts.join(","))
        }
        NuTypedef::Table(tab) => {
            let cols: Vec<std::string::String> = tab
                .columns
                .iter()
                .map(|c| format!("{}:{}", c.name.0, render_signature_typedef(&c.typedef)))
                .collect();
            format!("table<{}>", cols.join(","))
        }
        NuTypedef::List(l) => format!("list<{}>", render_signature_typedef(&l.element)),
    }
}

/// A record's fields in the compact spelling.
fn render_signature_fields(fields: &[NuRecordFieldTypedef]) -> std::string::String {
    let parts: Vec<std::string::String> = fields
        .iter()
        .map(|f| format!("{}:{}", f.name.0, render_signature_typedef(&f.typedef)))
        .collect();
    parts.join(",")
}

/// A signature group: a record as its fields, a table or nothing spelled whole.
pub fn signature_group(typedef: &NuTypedef) -> std::string::String {
    match typedef {
        NuTypedef::Record(record) => format!("<{}>", render_signature_fields(&record.fields)),
        other => format!("<{}>", render_signature_typedef(other)),
    }
}

/// Why an open record or table cannot type a definition's `data` at its top.
const OPEN_AT_TOP: &str = "an open `record<>` or `table<>` stands only nested; declare the fields";

/// A contract's `data` type: `nothing`, or a typed record or table.
pub(crate) fn data_typedef(shape: &nu::SyntaxShape) -> Result<NuTypedef, String> {
    match shape {
        nu::SyntaxShape::Nothing => Ok(NuTypedef::Nothing),
        nu::SyntaxShape::Record(columns) | nu::SyntaxShape::Table(columns)
            if columns.is_empty() =>
        {
            Err(OPEN_AT_TOP.to_string())
        }
        nu::SyntaxShape::Record(_) | nu::SyntaxShape::Table(_) => shape_typedef(shape),
        other => Err(format!(
            "typed `nothing`, `record<...>`, or `table<...>`, not `{other}`"
        )),
    }
}

/// A data type below its top; an open record or table stands here.
fn shape_typedef(shape: &nu::SyntaxShape) -> Result<NuTypedef, String> {
    use nu::SyntaxShape as Shape;
    let scalar = |scalar: NuScalarTypedef| Ok(NuTypedef::Scalar(scalar));
    match shape {
        Shape::Nothing => Ok(NuTypedef::Nothing),
        Shape::Int => scalar(NuScalarTypedef::Int),
        Shape::Float => scalar(NuScalarTypedef::Float),
        Shape::Number => scalar(NuScalarTypedef::Number),
        Shape::String => scalar(NuScalarTypedef::String),
        Shape::Boolean => scalar(NuScalarTypedef::Bool),
        Shape::DateTime => scalar(NuScalarTypedef::Datetime),
        Shape::Duration => scalar(NuScalarTypedef::Duration),
        Shape::Filesize => scalar(NuScalarTypedef::Filesize),
        Shape::Binary => scalar(NuScalarTypedef::Binary),
        Shape::Range => scalar(NuScalarTypedef::Range),
        Shape::GlobPattern => scalar(NuScalarTypedef::Glob),
        Shape::CellPath => scalar(NuScalarTypedef::CellPath),
        Shape::Filepath => scalar(NuScalarTypedef::Path),
        Shape::Directory => scalar(NuScalarTypedef::Directory),
        Shape::Record(fields) => Ok(NuTypedef::Record(NuRecordTypedef {
            fields: fields
                .iter()
                .map(|(name, shape)| {
                    Ok(NuRecordFieldTypedef {
                        name: FieldName(name.clone()),
                        typedef: shape_typedef(shape)?,
                    })
                })
                .collect::<Result<_, String>>()?,
        })),
        Shape::Table(columns) => Ok(NuTypedef::Table(NuTableTypedef {
            columns: columns
                .iter()
                .map(|(name, shape)| {
                    Ok(NuTableColumnTypedef {
                        name: ColumnName(name.clone()),
                        typedef: shape_typedef(shape)?,
                    })
                })
                .collect::<Result<_, String>>()?,
        })),
        Shape::List(element) => list_typedef(shape_typedef(element)?),
        Shape::OneOf(members) => Ok(NuTypedef::Oneof(NuOneofTypedef {
            members: members.iter().map(shape_typedef).collect::<Result<_, _>>()?,
        })),
        Shape::Any => Err("`any` is not a grammar type".to_string()),
        other => Err(format!("`{other}` is not a data type")),
    }
}

/// A contract's return type: `nothing`, or a typed record or table.
pub(crate) fn result_typedef(ty: &nu::Type) -> Result<NuTypedef, String> {
    match ty {
        nu::Type::Nothing => Ok(NuTypedef::Nothing),
        nu::Type::Record(_) | nu::Type::Table(_) => type_typedef(ty),
        other => Err(format!("typed `nothing`, `record<...>`, or `table<...>`, not `{other}`")),
    }
}

/// A return type at any depth; an open record or table is refused everywhere.
fn type_typedef(ty: &nu::Type) -> Result<NuTypedef, String> {
    use nu::Type as Ty;
    let scalar = |scalar: NuScalarTypedef| Ok(NuTypedef::Scalar(scalar));
    match ty {
        Ty::Nothing => Ok(NuTypedef::Nothing),
        Ty::Int => scalar(NuScalarTypedef::Int),
        Ty::Float => scalar(NuScalarTypedef::Float),
        Ty::Number => scalar(NuScalarTypedef::Number),
        Ty::String => scalar(NuScalarTypedef::String),
        Ty::Bool => scalar(NuScalarTypedef::Bool),
        Ty::Date => scalar(NuScalarTypedef::Datetime),
        Ty::Duration => scalar(NuScalarTypedef::Duration),
        Ty::Filesize => scalar(NuScalarTypedef::Filesize),
        Ty::Binary => scalar(NuScalarTypedef::Binary),
        Ty::Range => scalar(NuScalarTypedef::Range),
        Ty::Glob => scalar(NuScalarTypedef::Glob),
        Ty::CellPath => scalar(NuScalarTypedef::CellPath),
        Ty::Record(columns) | Ty::Table(columns) if columns.is_empty() => Err(
            "an open `record<>` or `table<>` is refused in a return type; declare the fields"
                .to_string(),
        ),
        Ty::Record(fields) => Ok(NuTypedef::Record(NuRecordTypedef {
            fields: fields
                .iter()
                .map(|(name, ty)| {
                    Ok(NuRecordFieldTypedef {
                        name: FieldName(name.clone()),
                        typedef: type_typedef(ty)?,
                    })
                })
                .collect::<Result<_, String>>()?,
        })),
        Ty::Table(columns) => Ok(NuTypedef::Table(NuTableTypedef {
            columns: columns
                .iter()
                .map(|(name, ty)| {
                    Ok(NuTableColumnTypedef {
                        name: ColumnName(name.clone()),
                        typedef: type_typedef(ty)?,
                    })
                })
                .collect::<Result<_, String>>()?,
        })),
        Ty::List(element) => list_typedef(type_typedef(element)?),
        Ty::OneOf(members) => Ok(NuTypedef::Oneof(NuOneofTypedef {
            members: members.iter().map(type_typedef).collect::<Result<_, _>>()?,
        })),
        Ty::Any => Err("`any` is not a grammar type".to_string()),
        other => Err(format!("`{other}` is not a data type")),
    }
}

/// A list of `element`, a list of tables refused.
fn list_typedef(element: NuTypedef) -> Result<NuTypedef, String> {
    let of_tables = match &element {
        NuTypedef::Table(_) => true,
        NuTypedef::List(inner) => matches!(*inner.element, NuTypedef::Record(_)),
        _ => false,
    };
    if of_tables {
        return Err("a list of tables is refused; hold each table in a record".to_string());
    }
    Ok(NuTypedef::List(NuListTypedef {
        element: Box::new(element),
    }))
}

/// Hold `value` to `typedef`: each record and table exactly its declared fields.
pub fn check_value(
    value: &nu::Value,
    typedef: &NuTypedef,
    at: &str,
) -> Result<(), String> {
    match typedef {
        NuTypedef::Nothing => conform(matches!(value, nu::Value::Nothing { .. }), value, "nothing", at),
        NuTypedef::Scalar(scalar) => conform(scalar_conforms(value, *scalar), value, scalar.name(), at),
        NuTypedef::Record(record) => {
            let Ok(fields) = value.as_record() else {
                return Err(mismatch(value, "record", at));
            };
            let declared: Vec<(&str, &NuTypedef)> = record
                .fields
                .iter()
                .map(|field| (field.name.0.as_str(), &field.typedef))
                .collect();
            check_fields(fields, &declared, at)
        }
        NuTypedef::Table(table) => {
            let Ok(rows) = value.as_list() else {
                return Err(mismatch(value, "table", at));
            };
            let declared: Vec<(&str, &NuTypedef)> = table
                .columns
                .iter()
                .map(|column| (column.name.0.as_str(), &column.typedef))
                .collect();
            for (index, row) in rows.iter().enumerate() {
                let row_at = format!("{at}[{index}]");
                let Ok(fields) = row.as_record() else {
                    return Err(mismatch(row, "record", &row_at));
                };
                check_fields(fields, &declared, &row_at)?;
            }
            Ok(())
        }
        NuTypedef::List(list) => {
            let Ok(items) = value.as_list() else {
                return Err(mismatch(value, "list", at));
            };
            for (index, item) in items.iter().enumerate() {
                check_value(item, &list.element, &format!("{at}[{index}]"))?;
            }
            Ok(())
        }
        NuTypedef::Oneof(oneof) => {
            if oneof.members.iter().any(|member| check_value(value, member, at).is_ok()) {
                Ok(())
            } else {
                Err(format!(
                    "{at}: a value typed {} matches none of `{}`",
                    value.get_type(),
                    render_nu_typedef(typedef),
                ))
            }
        }
    }
}

/// A record's fields held to the declared ones; none declared is an open record.
fn check_fields(
    record: &nu::Record,
    declared: &[(&str, &NuTypedef)],
    at: &str,
) -> Result<(), String> {
    if declared.is_empty() {
        return Ok(());
    }
    for (key, _) in record.iter() {
        if !declared.iter().any(|(name, _)| name == key) {
            return Err(format!("{at}.{key}: an undeclared field"));
        }
    }
    for (name, typedef) in declared {
        match record.get(*name) {
            Some(field) => check_value(field, typedef, &format!("{at}.{name}"))?,
            None => return Err(format!("{at}.{name}: a declared field is missing")),
        }
    }
    Ok(())
}

/// Whether a value's variant is the scalar's.
fn scalar_conforms(
    value: &nu::Value,
    scalar: NuScalarTypedef,
) -> bool {
    use NuScalarTypedef as S;
    matches!(
        (scalar, value),
        (S::Int, nu::Value::Int { .. })
            | (S::Float, nu::Value::Float { .. })
            | (S::Number, nu::Value::Int { .. } | nu::Value::Float { .. })
            | (S::String | S::Path | S::Directory, nu::Value::String { .. })
            | (S::Glob, nu::Value::Glob { .. } | nu::Value::String { .. })
            | (S::Bool, nu::Value::Bool { .. })
            | (S::Datetime, nu::Value::Date { .. })
            | (S::Duration, nu::Value::Duration { .. })
            | (S::Filesize, nu::Value::Filesize { .. })
            | (S::Binary, nu::Value::Binary { .. })
            | (S::Range, nu::Value::Range { .. })
            | (S::CellPath, nu::Value::CellPath { .. })
    )
}

/// `Ok` when the value conforms, else the mismatch at its path.
fn conform(
    conforms: bool,
    value: &nu::Value,
    expected: &str,
    at: &str,
) -> Result<(), String> {
    if conforms {
        Ok(())
    } else {
        Err(mismatch(value, expected, at))
    }
}

/// A value's type against the one expected at its path.
fn mismatch(
    value: &nu::Value,
    expected: &str,
    at: &str,
) -> String {
    format!("{at}: expected {expected}, got {}", value.get_type())
}

/// A typedef in nushell's spelling, an open record or table as `record<>`.
pub fn typedef_text(typedef: &NuTypedef) -> std::string::String {
    render_nu_typedef(typedef)
}
