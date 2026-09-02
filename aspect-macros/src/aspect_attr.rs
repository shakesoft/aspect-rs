//! Main transformation logic for the #[aspect] attribute macro.

use proc_macro2::TokenStream;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use syn::{Expr, ImplItem, Item, ItemFn, Result, Type};

use crate::codegen::{generate_aspect_wrapper, generate_async_aspect_wrapper};
use crate::parsing::AspectInfo;

/// Transforms a function by applying aspect weaving.
pub fn transform(aspect_expr: Expr, func: ItemFn) -> Result<TokenStream> {
    let mut aspect_info = AspectInfo::parse(aspect_expr)?;
    let type_name = extract_aspect_type_name(&aspect_info.aspect_expr);
    let is_async_aspect = type_name
        .as_deref()
        .map(is_async_aspect_type)
        .unwrap_or(false);

    if let Some(type_name) = type_name.as_deref() {
        aspect_info.has_custom_sync_around = has_custom_sync_around(type_name);
        aspect_info.has_custom_async_around = has_custom_async_around(type_name);
    }

    validate_aspect_usage(&func, &aspect_info.aspect_expr, is_async_aspect, &aspect_info)?;

    if func.sig.asyncness.is_some() && is_async_aspect {
        Ok(generate_async_aspect_wrapper(&aspect_info, &func))
    } else {
        Ok(generate_aspect_wrapper(&aspect_info, &func))
    }
}

fn validate_aspect_usage(
    func: &ItemFn,
    aspect_expr: &Expr,
    is_async_aspect: bool,
    aspect_info: &AspectInfo,
) -> Result<()> {
    if func.sig.asyncness.is_none() && is_async_aspect {
        return Err(syn::Error::new_spanned(
            aspect_expr,
            "async aspects can only be applied to async fn; sync fn must use a type that implements Aspect",
        ));
    }

    let returns_impl_trait = matches!(func.sig.output, syn::ReturnType::Type(_, ref ty) if matches!(ty.as_ref(), Type::ImplTrait(_)));

    if func.sig.asyncness.is_some() && !is_async_aspect && aspect_info.has_custom_sync_around {
        return Err(syn::Error::new_spanned(
            aspect_expr,
            "sync aspects that override around() cannot be applied to async fn; implement AsyncAspect or rely on before/after/after_error only",
        ));
    }

    if func.sig.asyncness.is_some() && is_async_aspect && returns_impl_trait && aspect_info.has_custom_async_around {
        return Err(syn::Error::new_spanned(
            aspect_expr,
            "async aspects that override around() cannot be applied to async fn returning impl Trait; use a concrete return type or rely on before/after/after_error only",
        ));
    }

    Ok(())
}

#[derive(Clone, Copy)]
enum Query {
    AsyncImpl,
    SyncAround,
    AsyncAround,
}

fn is_async_aspect_type(type_name: &str) -> bool {
    lookup(Query::AsyncImpl, type_name)
}

fn has_custom_sync_around(type_name: &str) -> bool {
    lookup(Query::SyncAround, type_name)
}

fn has_custom_async_around(type_name: &str) -> bool {
    lookup(Query::AsyncAround, type_name)
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

/// An `impl <trait> for <type>` found under `CARGO_MANIFEST_DIR`, reduced to
/// plain data. `syn` nodes carry `proc_macro` spans that are only valid inside
/// the expansion that parsed them, so the AST itself must never be cached.
struct ImplSummary {
    trait_name: String,
    type_name: String,
    has_around: bool,
}

/// Answers one `Query` about one aspect type.
fn lookup(query: Query, type_name: &str) -> bool {
    impl_summaries()
        .iter()
        .any(|summary| summary_matches(summary, query, type_name))
}

fn summary_matches(summary: &ImplSummary, query: Query, type_name: &str) -> bool {
    if summary.type_name != type_name {
        return false;
    }

    match query {
        Query::AsyncImpl => summary.trait_name == "AsyncAspect",
        Query::SyncAround => summary.trait_name == "Aspect" && summary.has_around,
        Query::AsyncAround => summary.trait_name == "AsyncAspect" && summary.has_around,
    }
}

/// Every trait impl under `CARGO_MANIFEST_DIR`, scanned once per process:
/// every `#[aspect]` in a crate interrogates the same unchanging source tree.
fn impl_summaries() -> &'static [ImplSummary] {
    static SUMMARIES: OnceLock<Vec<ImplSummary>> = OnceLock::new();

    SUMMARIES.get_or_init(|| {
        let Some(root) = std::env::var_os("CARGO_MANIFEST_DIR") else {
            return Vec::new();
        };

        let mut summaries = Vec::new();
        let mut stack = vec![PathBuf::from(root)];

        while let Some(dir) = stack.pop() {
            let Ok(entries) = fs::read_dir(&dir) else {
                continue;
            };

            for entry in entries.flatten() {
                // `DirEntry` already carries the type on every platform we
                // build for, so this avoids a `stat` syscall per entry.
                let Ok(file_type) = entry.file_type() else {
                    continue;
                };
                let path = entry.path();

                if file_type.is_dir() {
                    if !is_skipped_dir(&path) {
                        stack.push(path);
                    }
                    continue;
                }

                if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
                    continue;
                }

                let Ok(contents) = fs::read_to_string(&path) else {
                    continue;
                };
                let Ok(file) = syn::parse_file(&contents) else {
                    continue;
                };

                summaries.extend(summarize_impls(&file));
            }
        }

        summaries
    })
}

fn summarize_impls(file: &syn::File) -> Vec<ImplSummary> {
    file.items
        .iter()
        .filter_map(|item| {
            let Item::Impl(item_impl) = item else {
                return None;
            };
            let (_, trait_path, _) = item_impl.trait_.as_ref()?;
            let Type::Path(self_ty) = item_impl.self_ty.as_ref() else {
                return None;
            };

            Some(ImplSummary {
                trait_name: trait_path.segments.last()?.ident.to_string(),
                type_name: self_ty.path.segments.last()?.ident.to_string(),
                has_around: item_impl.items.iter().any(|impl_item| {
                    matches!(impl_item, ImplItem::Fn(method) if method.sig.ident == "around")
                }),
            })
        })
        .collect()
}

// ponytail: build output, VCS and vendor dirs hold no aspect impls but do hold
// most of a project's files - `target/` plus `node_modules/` were ~44k entries
// in the crate that motivated this, re-walked three times per `#[aspect]`. If
// generated code under `target/` ever needs scanning, take the roots from an
// env var rather than widening the walk back to everything.
fn is_skipped_dir(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with('.') || matches!(name, "target" | "node_modules"))
}

/// Test-only entry point: runs a `Query` against a single source string
/// through exactly the pipeline `lookup` uses.
#[cfg(test)]
fn source_matches(contents: &str, query: Query, type_name: &str) -> bool {
    let Ok(file) = syn::parse_file(contents) else {
        return false;
    };

    summarize_impls(&file)
        .iter()
        .any(|summary| summary_matches(summary, query, type_name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn skips_build_and_vendor_dirs() {
        assert!(is_skipped_dir(Path::new("proj/target")));
        assert!(is_skipped_dir(Path::new("proj/node_modules")));
        assert!(is_skipped_dir(Path::new("proj/.git")));
        assert!(!is_skipped_dir(Path::new("proj/src")));
    }

    #[test]
    fn detects_custom_sync_around_even_with_other_methods() {
        let source = r#"
            impl Aspect for Logger {
                fn before(&self, ctx: &JoinPoint) {
                    if ctx.args.is_empty() {
                        println!("empty");
                    }
                }

                fn around(&self, pjp: ProceedingJoinPoint) -> Result<Box<dyn Any>, AspectError> {
                    pjp.proceed()
                }
            }
        "#;

        assert!(source_matches(source, Query::SyncAround, "Logger"));
    }

    #[test]
    fn detects_async_impl_from_ast() {
        let source = r#"
            impl AsyncAspect for Logger1 {
                async fn before(&self, _ctx: &AsyncJoinPoint) {}
            }
        "#;

        assert!(source_matches(source, Query::AsyncImpl, "Logger1"));
    }

    #[test]
    fn detects_custom_async_around() {
        let source = r#"
            impl AsyncAspect for Logger1 {
                async fn around(&self, pjp: AsyncProceedingJoinPoint<'_>) -> Result<Box<dyn Any + Send + Sync>, AspectError> {
                    pjp.proceed().await
                }
            }
        "#;

        assert!(source_matches(source, Query::AsyncAround, "Logger1"));
    }

    #[test]
    fn rejects_async_aspect_on_sync_function() {
        let func: ItemFn = parse_quote! {
            fn demo() {}
        };

        let err = validate_aspect_usage(
            &func,
            &parse_quote!(Logger1),
            true,
            &AspectInfo::parse(parse_quote!(Logger1)).unwrap(),
        )
        .unwrap_err();
        assert!(
            err.to_string()
                .contains("async aspects can only be applied to async fn")
        );
    }

    #[test]
    fn rejects_custom_sync_around_on_async_function() {
        let func: ItemFn = parse_quote! {
            async fn demo() {}
        };
        let mut aspect_info = AspectInfo::parse(parse_quote!(Logger)).unwrap();
        aspect_info.has_custom_sync_around = true;

        let err = validate_aspect_usage(&func, &parse_quote!(Logger), false, &aspect_info)
            .unwrap_err();

        assert!(
            err.to_string()
                .contains("sync aspects that override around() cannot be applied to async fn")
        );
    }

    #[test]
    fn rejects_custom_async_around_on_impl_trait_async_function() {
        let func: ItemFn = parse_quote! {
            async fn demo() -> impl IntoResponse { 1 }
        };
        let mut aspect_info = AspectInfo::parse(parse_quote!(Logger1)).unwrap();
        aspect_info.has_custom_async_around = true;

        let err = validate_aspect_usage(&func, &parse_quote!(Logger1), true, &aspect_info)
            .unwrap_err();

        assert!(
            err.to_string().contains(
                "async aspects that override around() cannot be applied to async fn returning impl Trait"
            )
        );
    }
}
