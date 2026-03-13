//! Main transformation logic for the #[aspect] attribute macro.

use proc_macro2::TokenStream;
use std::fs;
use std::path::{Path, PathBuf};
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

fn is_async_aspect_type(type_name: &str) -> bool {
    let Some(manifest_dir) = std::env::var_os("CARGO_MANIFEST_DIR") else {
        return false;
    };

    contains_async_impl_recursively(Path::new(&manifest_dir), type_name)
}

fn has_custom_sync_around(type_name: &str) -> bool {
    let Some(manifest_dir) = std::env::var_os("CARGO_MANIFEST_DIR") else {
        return false;
    };

    contains_custom_sync_around_recursively(Path::new(&manifest_dir), type_name)
}

fn has_custom_async_around(type_name: &str) -> bool {
    let Some(manifest_dir) = std::env::var_os("CARGO_MANIFEST_DIR") else {
        return false;
    };

    contains_custom_async_around_recursively(Path::new(&manifest_dir), type_name)
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

        if file_contains_async_impl(&contents, type_name) {
            return true;
        }
    }

    false
}

fn contains_custom_sync_around_recursively(root: &Path, type_name: &str) -> bool {
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

        if file_contains_custom_sync_around(&contents, type_name) {
            return true;
        }
    }

    false
}

fn contains_custom_async_around_recursively(root: &Path, type_name: &str) -> bool {
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

        if file_contains_custom_async_around(&contents, type_name) {
            return true;
        }
    }

    false
}

fn file_contains_async_impl(contents: &str, type_name: &str) -> bool {
    let Ok(file) = syn::parse_file(contents) else {
        return false;
    };

    file.items.iter().any(|item| {
        let Item::Impl(item_impl) = item else {
            return false;
        };

        impl_targets_trait(item_impl, "AsyncAspect", type_name)
    })
}

fn file_contains_custom_sync_around(contents: &str, type_name: &str) -> bool {
    file_contains_custom_around(contents, "Aspect", type_name)
}

fn file_contains_custom_async_around(contents: &str, type_name: &str) -> bool {
    file_contains_custom_around(contents, "AsyncAspect", type_name)
}

fn file_contains_custom_around(contents: &str, trait_name: &str, type_name: &str) -> bool {
    let Ok(file) = syn::parse_file(contents) else {
        return false;
    };

    file.items.iter().any(|item| {
        let Item::Impl(item_impl) = item else {
            return false;
        };

        impl_targets_trait(item_impl, trait_name, type_name)
            && item_impl.items.iter().any(|impl_item| {
                matches!(
                    impl_item,
                    ImplItem::Fn(method) if method.sig.ident == "around"
                )
            })
    })
}

fn impl_targets_trait(item_impl: &syn::ItemImpl, trait_name: &str, type_name: &str) -> bool {
    let Some((_, trait_path, _)) = &item_impl.trait_ else {
        return false;
    };

    let trait_matches = trait_path
        .segments
        .last()
        .map(|segment| segment.ident == trait_name)
        .unwrap_or(false);
    if !trait_matches {
        return false;
    }

    match item_impl.self_ty.as_ref() {
        Type::Path(type_path) => type_path
            .path
            .segments
            .last()
            .map(|segment| segment.ident == type_name)
            .unwrap_or(false),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

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

        assert!(file_contains_custom_sync_around(source, "Logger"));
    }

    #[test]
    fn detects_async_impl_from_ast() {
        let source = r#"
            impl AsyncAspect for Logger1 {
                async fn before(&self, _ctx: &AsyncJoinPoint) {}
            }
        "#;

        assert!(file_contains_async_impl(source, "Logger1"));
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

        assert!(file_contains_custom_async_around(source, "Logger1"));
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
