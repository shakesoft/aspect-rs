//! Main transformation logic for the #[aspect] attribute macro.

use proc_macro2::TokenStream;
use std::fs;
use std::path::{Path, PathBuf};
use syn::{Expr, ItemFn, Result};

use crate::codegen::{generate_aspect_wrapper, generate_async_aspect_wrapper};
use crate::parsing::AspectInfo;

/// Transforms a function by applying aspect weaving.
pub fn transform(aspect_expr: Expr, func: ItemFn) -> Result<TokenStream> {
    let aspect_info = AspectInfo::parse(aspect_expr)?;
    if func.sig.asyncness.is_some() && is_async_aspect_expr(&aspect_info.aspect_expr) {
        Ok(generate_async_aspect_wrapper(&aspect_info, &func))
    } else {
        Ok(generate_aspect_wrapper(&aspect_info, &func))
    }
}

fn is_async_aspect_expr(expr: &Expr) -> bool {
    let Some(type_name) = extract_aspect_type_name(expr) else {
        return false;
    };

    let Some(manifest_dir) = std::env::var_os("CARGO_MANIFEST_DIR") else {
        return false;
    };

    contains_async_impl_recursively(Path::new(&manifest_dir), &type_name)
}

fn extract_aspect_type_name(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Path(path) => path.path.segments.last().map(|segment| segment.ident.to_string()),
        Expr::Call(call) => extract_aspect_type_name(&call.func),
        Expr::MethodCall(call) => extract_aspect_type_name(&call.receiver),
        Expr::Paren(paren) => extract_aspect_type_name(&paren.expr),
        Expr::Reference(reference) => extract_aspect_type_name(&reference.expr),
        _ => None,
    }
}

fn contains_async_impl_recursively(root: &Path, type_name: &str) -> bool {
    let mut stack = vec![PathBuf::from(root)];

    while let Some(path) = stack.pop() {
        let Ok(metadata) = fs::metadata(&path) else {
            continue;
        };

        if metadata.is_dir() {
            let Ok(entries) = fs::read_dir(&path) else {
                continue;
            };

            for entry in entries.flatten() {
                stack.push(entry.path());
            }
            continue;
        }

        if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            continue;
        }

        let Ok(contents) = fs::read_to_string(&path) else {
            continue;
        };

        if contains_async_impl(&contents, type_name) {
            return true;
        }
    }

    false
}

fn contains_async_impl(contents: &str, type_name: &str) -> bool {
    let needle = format!("impl AsyncAspect for {}", type_name);
    let mut remainder = contents;

    while let Some(index) = remainder.find(&needle) {
        let suffix = &remainder[index + needle.len()..];
        match suffix.chars().next() {
            None => return true,
            Some(ch) if !matches!(ch, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_') => return true,
            _ => {
                remainder = suffix;
            }
        }
    }

    false
}
