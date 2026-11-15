// =============================================================================
// AST-BASED TYPESCRIPT CONTRACT GENERATOR (`gen-ts`)
// =============================================================================
//
// Parses Rust source files under `crates/shared/src/domain/` and `crates/api/src/routes/`
// using `syn` (read-only) to synthesize strongly typed TypeScript interfaces,
// `ErrorCode` unions, and REST endpoint path/method tables inside
// `svelte-app/src/lib/api/`.
//
// ## Invariants
//
// - Read-only over Rust sources; writes only `.generated.ts` targets when called
//   via `write_generated_typescript()`.
// - Tested by `api::tests::generated_ts_is_up_to_date` so CI/clippy workflows
//   catch stale TypeScript contracts automatically.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

// `syn` - Parses Rust AST definitions without macro expansion side effects.
use syn::{
    Expr, Fields, GenericArgument, Item, ItemEnum, ItemStruct, Meta, PathArguments, Type,
};

/// Generated file pair produced by AST codegen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedTypeScript {
    pub types_ts: String,
    pub endpoints_ts: String,
}

/// Discovers repository root containing `crates/shared` and `svelte-app`.
pub fn workspace_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Generates TypeScript strings in-memory without writing files.
pub fn generate_typescript(root: &Path) -> Result<GeneratedTypeScript, String> {
    let domain_dir = root.join("crates/shared/src/domain");
    let routes_dir = root.join("crates/api/src/routes");

    let mut error_codes: Vec<String> = Vec::new();
    let mut type_defs: BTreeMap<String, String> = BTreeMap::new();

    let mut domain_files: Vec<PathBuf> = fs::read_dir(&domain_dir)
        .map_err(|e| format!("Failed reading {}: {e}", domain_dir.display()))?
        .filter_map(|res| res.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().and_then(|s| s.to_str()) == Some("rs"))
        .collect();
    domain_files.sort();

    for path in domain_files {
        let src = fs::read_to_string(&path)
            .map_err(|e| format!("Failed reading {}: {e}", path.display()))?;

        if path.file_name().and_then(|s| s.to_str()) == Some("error_code.rs") {
            extract_error_codes_from_source(&src, &mut error_codes);
        }

        let file = syn::parse_file(&src)
            .map_err(|e| format!("Failed parsing AST in {}: {e}", path.display()))?;

        for item in file.items {
            match item {
                Item::Struct(st) if is_public_serializable_struct(&st) => {
                    let rendered = render_struct_ts(&st);
                    type_defs.insert(st.ident.to_string(), rendered);
                }
                Item::Enum(en) if is_public_serializable_enum(&en) => {
                    let rendered = render_enum_ts(&en);
                    type_defs.insert(en.ident.to_string(), rendered);
                }
                _ => {}
            }
        }
    }

    let mut types_ts = String::new();
    types_ts.push_str("// =============================================================================\n");
    types_ts.push_str("// AUTO-GENERATED TYPESCRIPT CONTRACTS — DO NOT EDIT MANUALLY\n");
    types_ts.push_str("// Generated via: cargo run -p api --bin gen-ts\n");
    types_ts.push_str("// Source: crates/shared/src/domain/*.rs\n");
    types_ts.push_str("// =============================================================================\n\n");

    types_ts.push_str("export const ERROR_CODES = [\n");
    for code in &error_codes {
        types_ts.push_str(&format!("  '{code}',\n"));
    }
    types_ts.push_str("] as const;\n\n");
    types_ts.push_str("export type ErrorCode = (typeof ERROR_CODES)[number];\n\n");

    types_ts.push_str(
        "export type Status =\n  | { Success: 'Ok' | 'Created' }\n  | { Error: 'BadRequest' | 'Unauthorized' | 'Forbidden' | 'NotFound' | 'Conflict' | 'ServiceUnavailable' | 'InternalServerError' };\n\n",
    );

    for (name, body) in type_defs {
        if name == "ErrorCode" || name == "Status" {
            continue;
        }
        types_ts.push_str(&body);
        types_ts.push('\n');
    }

    let endpoints = extract_routes(&routes_dir)?;
    let mut endpoints_ts = String::new();
    endpoints_ts.push_str("// =============================================================================\n");
    endpoints_ts.push_str("// AUTO-GENERATED API ENDPOINT REGISTRY — DO NOT EDIT MANUALLY\n");
    endpoints_ts.push_str("// Generated via: cargo run -p api --bin gen-ts\n");
    endpoints_ts.push_str("// Source: crates/api/src/routes/*.rs\n");
    endpoints_ts.push_str("// =============================================================================\n\n");
    endpoints_ts.push_str("export type HttpMethod = 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE';\n\n");
    endpoints_ts.push_str("export interface ApiEndpointDef {\n  readonly method: HttpMethod;\n  readonly path: string;\n}\n\n");
    endpoints_ts.push_str("export const API_ENDPOINTS = {\n");
    for (key, (method, path)) in endpoints {
        endpoints_ts.push_str(&format!(
            "  {key}: {{ method: '{method}' as HttpMethod, path: '{path}' }},\n"
        ));
    }
    endpoints_ts.push_str("} as const;\n");

    Ok(GeneratedTypeScript {
        types_ts,
        endpoints_ts,
    })
}

/// Writes generated TypeScript bindings into `svelte-app/src/lib/api/`.
pub fn write_generated_typescript(root: &Path) -> Result<GeneratedTypeScript, String> {
    let generated = generate_typescript(root)?;
    let out_dir = root.join("svelte-app/src/lib/api");
    fs::create_dir_all(&out_dir)
        .map_err(|e| format!("Failed creating {}: {e}", out_dir.display()))?;

    fs::write(out_dir.join("types.generated.ts"), &generated.types_ts)
        .map_err(|e| format!("Failed writing types.generated.ts: {e}"))?;
    fs::write(out_dir.join("endpoints.generated.ts"), &generated.endpoints_ts)
        .map_err(|e| format!("Failed writing endpoints.generated.ts: {e}"))?;

    Ok(generated)
}

fn extract_error_codes_from_source(src: &str, out: &mut Vec<String>) {
    let mut in_macro = false;
    for raw_line in src.lines() {
        let line = raw_line.trim();
        if line.starts_with("define_error_codes!") {
            in_macro = true;
            continue;
        }
        if in_macro {
            if line == "}" {
                break;
            }
            if let Some((_, rhs)) = line.split_once("=>") {
                let mut parts = rhs.split('"');
                let _ = parts.next();
                if let Some(code) = parts.next() {
                    if !code.is_empty() && !out.contains(&code.to_string()) {
                        out.push(code.to_string());
                    }
                }
            }
        }
    }
}

fn is_public_serializable_struct(st: &ItemStruct) -> bool {
    matches!(st.vis, syn::Visibility::Public(_))
        && has_serde_derive(&st.attrs)
}

fn is_public_serializable_enum(en: &ItemEnum) -> bool {
    matches!(en.vis, syn::Visibility::Public(_))
        && has_serde_derive(&en.attrs)
}

fn has_serde_derive(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        if !attr.path().is_ident("derive") {
            return false;
        }
        if let Meta::List(list) = &attr.meta {
            let tokens = list.tokens.to_string();
            return tokens.contains("Serialize") || tokens.contains("Deserialize");
        }
        false
    })
}

fn extract_rename_all(attrs: &[syn::Attribute]) -> Option<String> {
    for attr in attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }
        if let Meta::List(list) = &attr.meta {
            let s = list.tokens.to_string();
            if let Some(idx) = s.find("rename_all") {
                let slice = &s[idx..];
                if let Some((_, rhs)) = slice.split_once('"') {
                    if let Some((val, _)) = rhs.split_once('"') {
                        return Some(val.to_string());
                    }
                }
            }
        }
    }
    None
}

fn apply_rename_all(variant: &str, rule: Option<&str>) -> String {
    match rule {
        Some("lowercase") => variant.to_lowercase(),
        Some("snake_case") => to_snake_case(variant),
        Some("SCREAMING_SNAKE_CASE") => to_snake_case(variant).to_uppercase(),
        _ => variant.to_string(),
    }
}

fn to_snake_case(name: &str) -> String {
    let mut out = String::new();
    for (idx, ch) in name.chars().enumerate() {
        if ch.is_uppercase() {
            if idx > 0 {
                out.push('_');
            }
            for lc in ch.to_lowercase() {
                out.push(lc);
            }
        } else {
            out.push(ch);
        }
    }
    out
}

fn render_struct_ts(st: &ItemStruct) -> String {
    let mut generics = String::new();
    if !st.generics.params.is_empty() {
        let params: Vec<String> = st
            .generics
            .type_params()
            .map(|p| p.ident.to_string())
            .collect();
        if !params.is_empty() {
            generics = format!("<{}>", params.join(", "));
        }
    }

    let mut out = format!("export interface {}{} {{\n", st.ident, generics);
    if let Fields::Named(fields) = &st.fields {
        for field in &fields.named {
            let Some(ident) = &field.ident else { continue };
            let (ts_ty, optional) = rust_type_to_ts(&field.ty);
            let opt_mark = if optional { "?" } else { "" };
            out.push_str(&format!("  {ident}{opt_mark}: {ts_ty};\n"));
        }
    }
    out.push_str("}\n");
    out
}

fn render_enum_ts(en: &ItemEnum) -> String {
    let rename_rule = extract_rename_all(&en.attrs);
    let mut variants = Vec::new();
    let all_unit = en.variants.iter().all(|v| matches!(v.fields, Fields::Unit));

    if all_unit {
        for v in &en.variants {
            let label = apply_rename_all(&v.ident.to_string(), rename_rule.as_deref());
            variants.push(format!("'{label}'"));
        }
        return format!("export type {} = {};\n", en.ident, variants.join(" | "));
    }

    // Check tagged enum (`#[serde(tag = "event", content = "payload")]`)
    for v in &en.variants {
        let label = apply_rename_all(&v.ident.to_string(), rename_rule.as_deref());
        match &v.fields {
            Fields::Unit => {
                variants.push(format!("{{ event: '{label}' }}"));
            }
            Fields::Unnamed(unnamed) if unnamed.unnamed.len() == 1 => {
                let (inner_ts, _) = rust_type_to_ts(&unnamed.unnamed[0].ty);
                variants.push(format!("{{ event: '{label}'; payload: {inner_ts} }}"));
            }
            Fields::Named(named) => {
                let fields: Vec<String> = named
                    .named
                    .iter()
                    .filter_map(|f| {
                        let name = f.ident.as_ref()?;
                        let (ty, _) = rust_type_to_ts(&f.ty);
                        Some(format!("{name}: {ty}"))
                    })
                    .collect();
                variants.push(format!(
                    "{{ event: '{label}'; payload: {{ {} }} }}",
                    fields.join("; ")
                ));
            }
            _ => variants.push(format!("{{ event: '{label}'; payload: unknown }}")),
        }
    }
    format!(
        "export type {} =\n  | {};\n",
        en.ident,
        variants.join("\n  | ")
    )
}

fn rust_type_to_ts(ty: &Type) -> (String, bool) {
    let Type::Path(tp) = ty else {
        return ("unknown".to_string(), false);
    };
    let Some(seg) = tp.path.segments.last() else {
        return ("unknown".to_string(), false);
    };
    let ident = seg.ident.to_string();

    if ident == "Option" {
        if let PathArguments::AngleBracketed(args) = &seg.arguments {
            if let Some(GenericArgument::Type(inner)) = args.args.first() {
                // Distinguish Option<Option<T>> (patch clear semantics) vs Option<T>
                if let Type::Path(inner_tp) = inner {
                    if inner_tp
                        .path
                        .segments
                        .last()
                        .is_some_and(|s| s.ident == "Option")
                    {
                        if let PathArguments::AngleBracketed(inner_args) =
                            &inner_tp.path.segments.last().expect("checked").arguments
                        {
                            if let Some(GenericArgument::Type(deep)) = inner_args.args.first() {
                                let (deep_ts, _) = rust_type_to_ts(deep);
                                return (format!("{deep_ts} | null"), true);
                            }
                        }
                    }
                }
                let (inner_ts, _) = rust_type_to_ts(inner);
                return (format!("{inner_ts} | null"), true);
            }
        }
        return ("unknown | null".to_string(), true);
    }

    if ident == "Vec" {
        if let PathArguments::AngleBracketed(args) = &seg.arguments {
            if let Some(GenericArgument::Type(inner)) = args.args.first() {
                let (inner_ts, _) = rust_type_to_ts(inner);
                return (format!("{inner_ts}[]"), false);
            }
        }
        return ("unknown[]".to_string(), false);
    }

    if ident == "HashMap" || ident == "BTreeMap" {
        if let PathArguments::AngleBracketed(args) = &seg.arguments {
            let mut iter = args.args.iter();
            let _key = iter.next();
            if let Some(GenericArgument::Type(val)) = iter.next() {
                let (val_ts, _) = rust_type_to_ts(val);
                return (format!("Record<string, {val_ts}>"), false);
            }
        }
        return ("Record<string, unknown>".to_string(), false);
    }

    match ident.as_str() {
        "String" | "str" | "DateTime" | "NaiveDate" | "Uuid" => ("string".to_string(), false),
        "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | "isize" | "usize"
        | "f32" | "f64" => ("number".to_string(), false),
        "bool" => ("boolean".to_string(), false),
        "Value" => ("unknown".to_string(), false),
        other => {
            if let PathArguments::AngleBracketed(args) = &seg.arguments {
                let mapped: Vec<String> = args
                    .args
                    .iter()
                    .filter_map(|arg| match arg {
                        GenericArgument::Type(t) => Some(rust_type_to_ts(t).0),
                        _ => None,
                    })
                    .collect();
                if !mapped.is_empty() {
                    return (format!("{other}<{}>", mapped.join(", ")), false);
                }
            }
            (other.to_string(), false)
        }
    }
}

fn extract_routes(routes_dir: &Path) -> Result<BTreeMap<String, (String, String)>, String> {
    let mut map = BTreeMap::new();
    if !routes_dir.exists() {
        return Ok(map);
    }

    let mut files: Vec<PathBuf> = fs::read_dir(routes_dir)
        .map_err(|e| format!("Failed reading {}: {e}", routes_dir.display()))?
        .filter_map(|res| res.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().and_then(|s| s.to_str()) == Some("rs"))
        .collect();
    files.sort();

    for path in files {
        let src = fs::read_to_string(&path)
            .map_err(|e| format!("Failed reading {}: {e}", path.display()))?;
        let file = syn::parse_file(&src)
            .map_err(|e| format!("Failed parsing {}: {e}", path.display()))?;

        let module = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("api")
            .to_string();

        for item in file.items {
            if let Item::Fn(func) = item {
                collect_route_calls_from_expr(&module, &func.block.stmts, &mut map);
            }
        }
    }

    Ok(map)
}

fn collect_route_calls_from_expr(
    module: &str,
    stmts: &[syn::Stmt],
    out: &mut BTreeMap<String, (String, String)>,
) {
    for stmt in stmts {
        if let syn::Stmt::Expr(expr, _) = stmt {
            visit_route_expr(module, expr, out);
        } else if let syn::Stmt::Local(local) = stmt {
            if let Some(init) = &local.init {
                visit_route_expr(module, &init.expr, out);
            }
        }
    }
}

fn visit_route_expr(
    module: &str,
    expr: &Expr,
    out: &mut BTreeMap<String, (String, String)>,
) {
    if let Expr::MethodCall(call) = expr {
        visit_route_expr(module, &call.receiver, out);
        if call.method == "route" && call.args.len() >= 2 {
            if let (Expr::Lit(path_lit), method_expr) = (&call.args[0], &call.args[1]) {
                if let syn::Lit::Str(lit_str) = &path_lit.lit {
                    let route_path = lit_str.value();
                    let methods = extract_http_methods_from_router(method_expr);
                    for (method, handler) in methods {
                        let key = format!("{}_{}", module.to_uppercase(), handler.to_uppercase());
                        out.insert(key, (method, route_path.clone()));
                    }
                }
            }
        }
    } else if let Expr::Block(block) = expr {
        collect_route_calls_from_expr(module, &block.block.stmts, out);
    }
}

fn extract_http_methods_from_router(expr: &Expr) -> Vec<(String, String)> {
    let mut found = Vec::new();
    collect_method_chain(expr, &mut found);
    found
}

fn collect_method_chain(expr: &Expr, out: &mut Vec<(String, String)>) {
    match expr {
        Expr::Call(call) => {
            if let Expr::Path(func_path) = &*call.func {
                if let Some(seg) = func_path.path.segments.last() {
                    let m = seg.ident.to_string().to_uppercase();
                    if matches!(m.as_str(), "GET" | "POST" | "PUT" | "PATCH" | "DELETE") {
                        let handler = call
                            .args
                            .first()
                            .and_then(extract_handler_name)
                            .unwrap_or_else(|| "handler".to_string());
                        out.push((m, handler));
                    }
                }
            }
        }
        Expr::MethodCall(mc) => {
            collect_method_chain(&mc.receiver, out);
            let m = mc.method.to_string().to_uppercase();
            if matches!(m.as_str(), "GET" | "POST" | "PUT" | "PATCH" | "DELETE") {
                let handler = mc
                    .args
                    .first()
                    .and_then(extract_handler_name)
                    .unwrap_or_else(|| "handler".to_string());
                out.push((m, handler));
            }
        }
        _ => {}
    }
}

fn extract_handler_name(expr: &Expr) -> Option<String> {
    if let Expr::Path(p) = expr {
        return p.path.segments.last().map(|s| s.ident.to_string());
    }
    None
}
