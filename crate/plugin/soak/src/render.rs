use crate::*;

type MemoryPartials = liquid::partials::EagerCompiler<liquid::partials::InMemorySource>;

const LIQUID: &str = "liquid";
const DOT_LIQUID: &str = ".liquid";

pub(crate) fn build_memory_partials(render_dir: &Path) -> SoakResult<MemoryPartials> {
    let mut partials = MemoryPartials::empty();
    if !render_dir.is_dir() {
        return Ok(partials)
    }
        
    let walker = dir_walker::Walker::new(render_dir)
        .skip_dotted()
        .walk_dir()
        .map_err(|e| CorkError::path_io(e, render_dir, PathOp::ListDir, None))?;

    for item in walker.into_iter() {
        let path = item.dirent.path();
        
        if !path.is_file() {
            continue;
        } else if path.extension().is_some_and(|ext| ext != LIQUID) {
            continue;
        }

        let name = path.strip_prefix(render_dir)
            .expect("parent")
            .to_str()
            .ok_or_else(|| CorkError::utf8(render_dir, None))?
            .trim_end_matches(DOT_LIQUID)
            .to_string();

        let contents = fs::read_to_string(&path)
            .map_err(|e| CorkError::path_io(e, path, PathOp::ReadFile, None))?;
        
        partials.add(name, contents);
    }

    Ok(partials)
}

pub(crate) fn render_dir<'a>(
    src_dir: PathSpan<'a>,
    dst_dir: PathSpan<'a>,
    _exclude: Option<&[&str]>, //todo
    _include: Option<&[&str]>, //todo
    render_dir: Option<&Path>,
    fill: &nu_protocol::Record,
) -> SoakResult<nu_protocol::Value> {
    let render_dir = match render_dir {
        Some(dir) => src_dir.path.join(dir),
        None => src_dir.path.join(LIQUID),
    };

    if !src_dir.path.is_dir() {
        return CorkError::err_path_not_found(src_dir.path, PathKind::Dir, src_dir.span)?;
    }
    if dst_dir.path.exists() {
        if !dst_dir.path.is_dir() {
            return CorkError::err_path_not_found(dst_dir.path, PathKind::Dir, dst_dir.span)?;
        }
    } else {
        fs::create_dir_all(dst_dir.path)
            .map_err(|e| CorkError::path_io(e, dst_dir.path, PathOp::CreateDir, dst_dir.span))?;
    }

    let parser = {
        let mut builder = liquid::ParserBuilder::with_stdlib();

        if render_dir.is_dir() {
            let partials = build_memory_partials(&render_dir)?;
            builder = builder.partials(partials);
        }

        builder.build()
            .map_err(|source| SoakError::LiquidParse { source })?
    };
    
    let fill = record_to_liquid_object(fill)?;

    let walker = dir_walker::Walker::new(src_dir.path)
        .skip_directories(&[render_dir.as_path()])
        .skip_dotted()
        .walk_dir()
        .map_err(|e| CorkError::path_io(e, src_dir.path, PathOp::ListDir, src_dir.span))?;

    for item in walker.into_iter() {
        let src_path = item.dirent.path();
        if src_path == src_dir.path || src_path.starts_with(&render_dir) {
            continue;
        }
        
        let meta = item.dirent.metadata()
            .map_err(|e| CorkError::path_io(e, &src_path, PathOp::DirMeta, src_dir.span))?;
        let rel_path = src_path.strip_prefix(src_dir.path).expect("relative exists")
            .to_path_buf();
        let mut dst_path = dst_dir.path.join(&rel_path);
        if meta.is_dir() {
            if !dst_path.exists() {
                fs::create_dir_all(&dst_path)
                    .map_err(|e| CorkError::path_io(e, dst_path, PathOp::CreateDir, dst_dir.span))?;
            }
        } else if meta.is_file() {
            let parent_dir = dst_path.parent().expect("parent");
            if !parent_dir.exists() {
                fs::create_dir_all(parent_dir)
                    .map_err(|e| CorkError::path_io(e, &dst_path, PathOp::CreateDir, dst_dir.span))?;
            }
            if rel_path.extension().is_some_and(|ext| ext == LIQUID) {
                dst_path.set_extension("");
                let template = parser.parse_file(&src_path).unwrap();
                let file = File::create(&dst_path)
                    .map_err(|e| CorkError::path_io(e, dst_path, PathOp::WriteFile, dst_dir.span))?;
                let mut writer = BufWriter::new(file);
                template.render_to(&mut writer, &fill)
                    .map_err(|source| SoakError::LiquidParse { source })?;
            } else {
                fs::copy(&src_path, &dst_path)
                    .map_err(|e| CorkError::path_copy(e, PathKind::File, src_path, dst_path, None))?;
            }
        }
    }

    Ok(nu_protocol::Value::nothing(NOSPAN))
}

pub(crate) fn render_str(
    s: &str,
    fill: &nu_protocol::Record,
) -> SoakResult<nu_protocol::Value> {
    let template = liquid::ParserBuilder::with_stdlib()
        .build()
        .map_err(|source| SoakError::LiquidParse { source })?
        .parse(s)
        .map_err(|source| SoakError::LiquidParse { source })?;

    let fill = record_to_liquid_object(fill)?;
    let out = template.render(&fill)
        .map_err(|source| SoakError::LiquidParse { source })?;
    
    Ok(nu_protocol::Value::string(out, NOSPAN))
}

fn record_to_liquid_object(record: &nu_protocol::Record) -> SoakResult<liquid::Object> {
    let mut object = liquid::Object::new();
    for (key, value) in record.iter() {
        let k = key.clone().into();
        let v = value_to_liquid_value(value)?;
        object.insert(k, v);
    }
    
    Ok(object)
}

fn value_to_liquid_value(value: &nu_protocol::Value) -> SoakResult<liquid::model::Value> {
    use nu_protocol::Value as NuValue;
    use liquid::model::Value as LiquidValue;
    
    match value {
        NuValue::Nothing { .. } => Ok(LiquidValue::Nil),
        // Scalar::Integer(i64)
        NuValue::Int { val,.. } => Ok(LiquidValue::scalar(*val)),
        // Scalar::Float(f64)
        NuValue::Float { val,.. } => Ok(LiquidValue::scalar(*val)),
        // Scalar::Bool(bool)
        NuValue::Bool { val,.. } => Ok(LiquidValue::scalar(*val)),
        // Scalar::Str(private equivalent of Cow<str>)
        NuValue::String { val,.. } => Ok(LiquidValue::scalar(val.clone())),
        NuValue::Glob { val,.. } => Ok(LiquidValue::scalar(val.clone())),
        NuValue::Filesize { val,.. } => Ok(LiquidValue::scalar(val.to_string())),
        NuValue::Range { val,.. } => Ok(LiquidValue::scalar(val.to_string())),
        NuValue::CellPath { val,.. } => Ok(LiquidValue::scalar(val.to_string())),
        v @ NuValue::Binary { .. } => {
            let utf8 = v.coerce_string()
                .map_err(|_| SoakError::LiquidValue { invalid: error::NuType::Binary, span: v.span() })?;
            Ok(LiquidValue::scalar(utf8))
        },
        // Array
        NuValue::List { vals,.. } => {
            let vals: Vec<LiquidValue> = vals.iter()
                .map(value_to_liquid_value)
                .collect::<SoakResult<_>>()?;
            Ok(LiquidValue::array(vals))
        },
        // Object
        NuValue::Record { val, .. } => {
            let v = record_to_liquid_object(val)?;
            Ok(LiquidValue::Object(v))
        },
        NuValue::Duration { val,.. } => {
            let v = Duration::from_nanos(*val as u64);
            let v = humantime::Duration::from(v);
            Ok(LiquidValue::scalar(v.to_string()))
        },
        NuValue::Date { val, internal_span,.. } => {
            let v = val.format("%Y-%m-%d %H:%M:%S.%f %z").to_string();
            let v = liquid::model::DateTime::from_str(&v)
                .ok_or_else(|| SoakError::LiquidValue { invalid: error::NuType::Date, span: *internal_span })?;
            Ok(LiquidValue::scalar(v))
        },
        NuValue::Custom { val, internal_span,.. } => {
            let v = val.to_base_value(*internal_span)
                .map_err(|_| SoakError::LiquidValue { invalid: error::NuType::Custom, span: *internal_span })?;
            // guard against infinite recursion, beyond nu's contract for to_base_value()
            if matches!(v, NuValue::Custom {..}) {
                return Err(SoakError::LiquidValue { invalid: error::NuType::Custom, span: *internal_span });
            }
            let v = value_to_liquid_value(&v)?;
            Ok(v)
        },
        NuValue::Closure { internal_span,.. } => Err(SoakError::LiquidValue { invalid: error::NuType::Closure, span: *internal_span }),
        NuValue::Error { internal_span,.. } => Err(SoakError::LiquidValue { invalid: error::NuType::Error, span: *internal_span }),
    }
}
