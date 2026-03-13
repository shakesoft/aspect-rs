//! Code generation utilities for aspect weaving.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Expr, ItemFn, Pat, ReturnType, Type};

use crate::parsing::AspectInfo;

fn generate_sync_args(param_names: &[&Box<syn::Pat>]) -> TokenStream {
    quote! { vec![#(Box::new(#param_names.clone()) as Box<dyn Any>),*] }
}

fn generate_async_arg_captures(debug_arg_idents: &[syn::Ident]) -> (Vec<syn::Ident>, TokenStream) {
    let capture_idents: Vec<_> = debug_arg_idents
        .iter()
        .enumerate()
        .map(|(idx, _)| format_ident!("__aspect_arg_{idx}"))
        .collect();

    let captures = quote! {
        #(let #capture_idents = #debug_arg_idents.clone();)*
    };

    (capture_idents, captures)
}

fn generate_async_args(capture_idents: &[syn::Ident]) -> TokenStream {
    quote! { vec![#(Box::new(#capture_idents.clone()) as Box<dyn Any>),*] }
}

fn generate_async_send_args(capture_idents: &[syn::Ident]) -> TokenStream {
    quote! { vec![#(Box::new(#capture_idents.clone()) as Box<dyn Any + Send + Sync>),*] }
}

/// Generates the aspect-woven code for a function.
pub fn generate_aspect_wrapper(aspect_info: &AspectInfo, func: &ItemFn) -> TokenStream {
    let original_fn = func;
    let fn_name = &func.sig.ident;
    let fn_vis = &func.vis;
    let fn_inputs = &func.sig.inputs;
    let fn_output = &func.sig.output;
    let fn_generics = &func.sig.generics;
    let fn_where_clause = &func.sig.generics.where_clause;
    let fn_asyncness = &func.sig.asyncness;

    let aspect_expr = &aspect_info.aspect_expr;

    let original_fn_name =
        syn::Ident::new(&format!("__aspect_original_{}", fn_name), fn_name.span());

    let mut original_fn_renamed = original_fn.clone();
    original_fn_renamed.sig.ident = original_fn_name.clone();
    original_fn_renamed.vis = syn::Visibility::Inherited;

    let param_names: Vec<_> = func
        .sig
        .inputs
        .iter()
        .filter_map(|arg| {
            if let syn::FnArg::Typed(pat_type) = arg {
                Some(&pat_type.pat)
            } else {
                None
            }
        })
        .collect();

    let mut debug_arg_idents: Vec<syn::Ident> = Vec::new();
    for arg in &func.sig.inputs {
        if let syn::FnArg::Typed(pat_type) = arg {
            collect_pat_idents(&pat_type.pat, &mut debug_arg_idents);
        }
    }

    let (return_type, is_result) = match fn_output {
        ReturnType::Default => (quote! { () }, false),
        ReturnType::Type(_, ty) => (quote! { #ty }, is_result_type(ty)),
    };
    let aspect_call = if fn_asyncness.is_some() {
        generate_async_around_call(
            aspect_expr,
            &original_fn_name,
            fn_name,
            &param_names,
            &debug_arg_idents,
            &return_type,
            is_result,
        )
    } else {
        generate_sync_around_call(
            aspect_expr,
            &original_fn_name,
            fn_name,
            &param_names,
            &return_type,
            is_result,
        )
    };

    quote! {
        #original_fn_renamed

        #fn_vis #fn_asyncness fn #fn_name #fn_generics(#fn_inputs) #fn_output #fn_where_clause {
            #aspect_call
        }
    }
}

/// Generates aspect weaving code for synchronous functions using around advice.
fn generate_sync_around_call(
    aspect_expr: &Expr,
    original_fn_name: &syn::Ident,
    fn_name: &syn::Ident,
    param_names: &[&Box<syn::Pat>],
    _return_type: &TokenStream,
    is_result: bool,
) -> TokenStream {
    let fn_name_str = fn_name.to_string();
    let args_expr = generate_sync_args(param_names);

    if is_result {
        // For Result types, unwrap and propagate errors properly
        quote! {
            use ::aspect_core::prelude::*;
            use ::std::any::Any;

            let __aspect = #aspect_expr;
            // Create ProceedingJoinPoint that wraps the original function
            let __args = #args_expr;
            let __context = JoinPoint {
                function_name: #fn_name_str,
                module_path: module_path!(),
                location: Location {
                    file: file!(),
                    line: line!(),
                },
                args: __args,
            };
            let __pjp = ProceedingJoinPoint::new(
                || {
                    match #original_fn_name(#(#param_names),*) {
                        Ok(__val) => Ok(Box::new(__val) as Box<dyn Any>),
                        Err(__err) => Err(AspectError::execution(format!("{:?}", __err))),
                    }
                },
                __context,
            );

            // Call the aspect's around method
            match __aspect.around(__pjp) {
                Ok(__boxed_result) => {
                    // Downcast the result back to the original Ok type
                    let __inner = *__boxed_result
                        .downcast::<_>()
                        .expect("aspect around() returned wrong type");
                    Ok(__inner)
                }
                Err(__err) => {
                    // Convert AspectError back to the function's error type
                    Err(format!("{:?}", __err).into())
                }
            }
        }
    } else {
        // For non-Result types
        quote! {
            use ::aspect_core::prelude::*;
            use ::std::any::Any;

            let __aspect = #aspect_expr;
            // Create ProceedingJoinPoint that wraps the original function
            let __args = #args_expr;
            let __context = JoinPoint {
                function_name: #fn_name_str,
                module_path: module_path!(),
                location: Location {
                    file: file!(),
                    line: line!(),
                },
                args: __args,
            };
            let __pjp = ProceedingJoinPoint::new(
                || {
                    let __result = #original_fn_name(#(#param_names),*);
                    Ok(Box::new(__result) as Box<dyn Any>)
                },
                __context,
            );

            // Call the aspect's around method
            match __aspect.around(__pjp) {
                Ok(__boxed_result) => {
                    // Downcast the result back to the original type
                    *__boxed_result
                        .downcast::<_>()
                        .expect("aspect around() returned wrong type")
                }
                Err(__err) => {
                    panic!("aspect around() failed: {:?}", __err);
                }
            }
        }
    }
}

/// Generates aspect weaving code for asynchronous functions using around advice.
fn generate_async_around_call(
    aspect_expr: &Expr,
    original_fn_name: &syn::Ident,
    fn_name: &syn::Ident,
    param_names: &[&Box<syn::Pat>],
    debug_arg_idents: &[syn::Ident],
    _return_type: &TokenStream,
    is_result: bool,
) -> TokenStream {
    let fn_name_str = fn_name.to_string();
    let (capture_idents, capture_bindings) = generate_async_arg_captures(debug_arg_idents);
    let args_expr = generate_async_args(&capture_idents);

    if is_result {
        return quote! {
            use ::aspect_core::prelude::*;
            use ::std::any::Any;

            let __aspect = #aspect_expr;
            #capture_bindings
            __aspect.before(&JoinPoint {
                function_name: #fn_name_str,
                module_path: module_path!(),
                location: Location {
                    file: file!(),
                    line: ::core::line!(),
                },
                args: #args_expr,
            });

            match #original_fn_name(#(#param_names),*).await {
                Ok(__value) => {
                    let __after_context = JoinPoint {
                        function_name: #fn_name_str,
                        module_path: module_path!(),
                        location: Location {
                            file: file!(),
                            line: ::core::line!(),
                        },
                        args: #args_expr,
                    };
                    __aspect.after(&__after_context, &__value as &dyn Any);
                    Ok(__value)
                }
                Err(__err) => {
                    let __error_context = JoinPoint {
                        function_name: #fn_name_str,
                        module_path: module_path!(),
                        location: Location {
                            file: file!(),
                            line: ::core::line!(),
                        },
                        args: #args_expr,
                    };
                    let __aspect_err = AspectError::execution(format!("{:?}", __err));
                    __aspect.after_error(&__error_context, &__aspect_err);
                    Err(__err)
                }
            }
        };
    } else {
        quote! {
            use ::aspect_core::prelude::*;
            use ::std::any::Any;

            let __aspect = #aspect_expr;
            #capture_bindings
            __aspect.before(&JoinPoint {
                function_name: #fn_name_str,
                module_path: module_path!(),
                location: Location {
                    file: file!(),
                    line: ::core::line!(),
                },
                args: #args_expr,
            });

            let __result = #original_fn_name(#(#param_names),*).await;
            let __after_context = JoinPoint {
                function_name: #fn_name_str,
                module_path: module_path!(),
                location: Location {
                    file: file!(),
                    line: ::core::line!(),
                },
                args: #args_expr,
            };
            __aspect.after(&__after_context, &__result as &dyn Any);
            __result
        }
    }
}
/// Checks if a type is a Result type.
fn is_result_type(ty: &syn::Type) -> bool {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "Result";
        }
    }
    false
}

/// Recursively collects identifier names from patterns.
fn collect_pat_idents(pat: &Pat, out: &mut Vec<syn::Ident>) {
    match pat {
        Pat::Ident(pat_ident) => out.push(pat_ident.ident.clone()),
        Pat::Reference(p) => collect_pat_idents(&p.pat, out),
        Pat::Type(p) => collect_pat_idents(&p.pat, out),
        Pat::Tuple(p) => {
            for elem in &p.elems {
                collect_pat_idents(elem, out);
            }
        }
        Pat::TupleStruct(p) => {
            for elem in &p.elems {
                collect_pat_idents(elem, out);
            }
        }
        Pat::Struct(p) => {
            for field in &p.fields {
                collect_pat_idents(&field.pat, out);
            }
        }
        Pat::Slice(p) => {
            for elem in &p.elems {
                collect_pat_idents(elem, out);
            }
        }
        Pat::Paren(p) => collect_pat_idents(&p.pat, out),
        Pat::Or(p) => {
            if let Some(first) = p.cases.first() {
                collect_pat_idents(first, out);
            }
        }
        _ => {}
    }
}

/// Generates the async-aspect wrapper code for an async function.
pub fn generate_async_aspect_wrapper(aspect_info: &AspectInfo, func: &ItemFn) -> TokenStream {
    let original_fn = func;
    let fn_name = &func.sig.ident;
    let fn_vis = &func.vis;
    let fn_inputs = &func.sig.inputs;
    let fn_output = &func.sig.output;
    let fn_generics = &func.sig.generics;
    let fn_where_clause = &func.sig.generics.where_clause;

    let aspect_expr = &aspect_info.aspect_expr;

    let original_fn_name = syn::Ident::new(
        &format!("__async_aspect_original_{}", fn_name),
        fn_name.span(),
    );

    let mut original_fn_renamed = original_fn.clone();
    original_fn_renamed.sig.ident = original_fn_name.clone();
    original_fn_renamed.vis = syn::Visibility::Inherited;

    let param_names: Vec<_> = func
        .sig
        .inputs
        .iter()
        .filter_map(|arg| {
            if let syn::FnArg::Typed(pat_type) = arg {
                Some(&pat_type.pat)
            } else {
                None
            }
        })
        .collect();

    let mut debug_arg_idents: Vec<syn::Ident> = Vec::new();
    for arg in &func.sig.inputs {
        if let syn::FnArg::Typed(pat_type) = arg {
            collect_pat_idents(&pat_type.pat, &mut debug_arg_idents);
        }
    }

    let (return_type, is_result) = match fn_output {
        ReturnType::Default => (quote! { () }, false),
        ReturnType::Type(_, ty) => (quote! { #ty }, is_result_type(ty)),
    };
    let returns_impl_trait = match fn_output {
        ReturnType::Type(_, ty) => matches!(ty.as_ref(), Type::ImplTrait(_)),
        ReturnType::Default => false,
    };

    let aspect_call = generate_async_aspect_call(
        aspect_expr,
        &original_fn_name,
        fn_name,
        &param_names,
        &debug_arg_idents,
        &return_type,
        is_result,
        returns_impl_trait,
        aspect_info.has_custom_async_around,
    );

    quote! {
        #original_fn_renamed

        #fn_vis async fn #fn_name #fn_generics(#fn_inputs) #fn_output #fn_where_clause {
            #aspect_call
        }
    }
}

fn generate_async_aspect_call(
    aspect_expr: &Expr,
    original_fn_name: &syn::Ident,
    fn_name: &syn::Ident,
    param_names: &[&Box<syn::Pat>],
    debug_arg_idents: &[syn::Ident],
    _return_type: &TokenStream,
    is_result: bool,
    returns_impl_trait: bool,
    has_custom_async_around: bool,
) -> TokenStream {
    let fn_name_str = fn_name.to_string();
    let (capture_idents, capture_bindings) = generate_async_arg_captures(debug_arg_idents);
    let args_expr = generate_async_send_args(&capture_idents);

    if !has_custom_async_around {
        if is_result {
            return quote! {
                use ::aspect_core::prelude::*;
                use ::std::any::Any;

                let __aspect = #aspect_expr;
                #capture_bindings

                __aspect.before(&AsyncJoinPoint {
                    function_name: #fn_name_str,
                    module_path: module_path!(),
                    location: Location {
                        file: file!(),
                        line: ::core::line!(),
                    },
                    args: #args_expr,
                }).await;

                match #original_fn_name(#(#param_names),*).await {
                    Ok(__value) => {
                        let __after_context = AsyncJoinPoint {
                            function_name: #fn_name_str,
                            module_path: module_path!(),
                            location: Location {
                                file: file!(),
                                line: ::core::line!(),
                            },
                            args: #args_expr,
                        };
                        __aspect.after(&__after_context, &__value as &(dyn Any + Send + Sync)).await;
                        Ok(__value)
                    }
                    Err(__err) => {
                        let __error_context = AsyncJoinPoint {
                            function_name: #fn_name_str,
                            module_path: module_path!(),
                            location: Location {
                                file: file!(),
                                line: ::core::line!(),
                            },
                            args: #args_expr,
                        };
                        let __aspect_err = AspectError::execution(format!("{:?}", __err));
                        __aspect.after_error(&__error_context, &__aspect_err).await;
                        Err(__err)
                    }
                }
            };
        }

        return quote! {
            use ::aspect_core::prelude::*;
            use ::std::any::Any;

            let __aspect = #aspect_expr;
            #capture_bindings

            __aspect.before(&AsyncJoinPoint {
                function_name: #fn_name_str,
                module_path: module_path!(),
                location: Location {
                    file: file!(),
                    line: ::core::line!(),
                },
                args: #args_expr,
            }).await;

            let __result = #original_fn_name(#(#param_names),*).await;
            let __after_context = AsyncJoinPoint {
                function_name: #fn_name_str,
                module_path: module_path!(),
                location: Location {
                    file: file!(),
                    line: ::core::line!(),
                },
                args: #args_expr,
            };
            __aspect.after(&__after_context, &__result as &(dyn Any + Send + Sync)).await;
            __result
        };
    }

    if is_result {
        quote! {
            use ::aspect_core::prelude::*;
            use ::std::any::Any;

            let __aspect = #aspect_expr;
            #capture_bindings

            let __context = AsyncJoinPoint {
                function_name: #fn_name_str,
                module_path: module_path!(),
                location: Location {
                    file: file!(),
                    line: ::core::line!(),
                },
                args: #args_expr,
            };

            let __pjp = AsyncProceedingJoinPoint::new(
                move || {
                    Box::pin(async move {
                        match #original_fn_name(#(#param_names),*).await {
                            Ok(__val) => Ok(Box::new(__val) as Box<dyn Any + Send + Sync>),
                            Err(__err) => Err(AspectError::execution(format!("{:?}", __err))),
                        }
                    })
                },
                __context,
            );

            match __aspect.around(__pjp).await {
                Ok(__boxed_result) => {
                    let __result = *__boxed_result
                        .downcast::<_>()
                        .expect("async aspect around() returned wrong type");
                    Ok(__result)
                }
                Err(__err) => Err(format!("{:?}", __err).into())
            }
        }
    } else if returns_impl_trait {
        quote! {
            compile_error!("async aspects that override around() cannot be applied to async fn returning impl Trait; use a concrete return type or rely on before/after/after_error only");
        }
    } else {
        quote! {
            use ::aspect_core::prelude::*;
            use ::std::any::Any;

            fn __async_aspect_take_result<T: 'static + Send>(boxed: Box<dyn Any + Send + Sync>) -> T {
                *boxed
                    .downcast::<T>()
                    .expect("async aspect around() returned wrong type")
            }

            let __aspect = #aspect_expr;
            #capture_bindings

            let __context = AsyncJoinPoint {
                function_name: #fn_name_str,
                module_path: module_path!(),
                location: Location {
                    file: file!(),
                    line: ::core::line!(),
                },
                args: #args_expr,
            };

            let __pjp = AsyncProceedingJoinPoint::new(
                move || {
                    Box::pin(async move {
                        let __result = #original_fn_name(#(#param_names),*).await;
                        Ok(Box::new(__result) as Box<dyn Any + Send + Sync>)
                    })
                },
                __context,
            );

            match __aspect.around(__pjp).await {
                Ok(__boxed_result) => {
                    let __result = __async_aspect_take_result(__boxed_result);
                    __result
                }
                Err(__err) => panic!("async aspect around() failed: {:?}", __err)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_is_result_type() {
        let result_type: syn::Type = parse_quote!(Result<i32, String>);
        assert!(is_result_type(&result_type));

        let non_result_type: syn::Type = parse_quote!(i32);
        assert!(!is_result_type(&non_result_type));
    }

    #[test]
    fn test_collect_pat_idents_tuple_struct() {
        let pat: Pat = parse_quote!(Query(params));
        let mut out = Vec::new();
        collect_pat_idents(&pat, &mut out);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0], "params");
    }

    #[test]
    fn test_generate_async_args_uses_cloned_original_values() {
        let args = vec![syn::Ident::new("a", proc_macro2::Span::call_site())];
        let tokens = generate_async_args(&args).to_string();

        assert!(tokens.contains("Box :: new (a . clone ()) as Box < dyn Any >"));
    }

    #[test]
    fn test_generate_async_wrapper_uses_around() {
        let func: ItemFn = parse_quote! {
            async fn demo(a: i32) -> i32 { a + 1 }
        };
        let aspect_info = AspectInfo::parse(parse_quote!(Logger)).unwrap();
        let tokens = generate_aspect_wrapper(&aspect_info, &func).to_string();

        assert!(tokens.contains("__aspect . before (& JoinPoint"));
        assert!(tokens.contains("__aspect . after (& __after_context"));
        assert!(!tokens.contains("tokio :: task :: block_in_place"));
    }

    #[test]
    fn test_generate_async_aspect_wrapper_uses_async_joinpoint() {
        let func: ItemFn = parse_quote! {
            async fn demo(a: i32) -> i32 { a + 1 }
        };
        let mut aspect_info = AspectInfo::parse(parse_quote!(Logger)).unwrap();
        aspect_info.has_custom_async_around = true;
        let tokens = generate_async_aspect_wrapper(&aspect_info, &func).to_string();

        assert!(tokens.contains("AsyncProceedingJoinPoint :: new"));
        assert!(tokens.contains("__aspect . around (__pjp) . await"));
        assert!(tokens.contains("Box < dyn Any + Send + Sync >"));
    }

    #[test]
    fn test_generate_async_aspect_wrapper_uses_around_for_impl_trait() {
        let func: ItemFn = parse_quote! {
            async fn demo(a: i32) -> impl IntoResponse { a + 1 }
        };
        let aspect_info = AspectInfo::parse(parse_quote!(Logger)).unwrap();
        let tokens = generate_async_aspect_wrapper(&aspect_info, &func).to_string();

        assert!(!tokens.contains("AsyncProceedingJoinPoint :: new"));
        assert!(!tokens.contains("__aspect . around (__pjp) . await"));
        assert!(tokens.contains("__aspect . before (& AsyncJoinPoint"));
        assert!(tokens.contains("__aspect . after (& __after_context"));
        assert!(!tokens.contains("axum :: response"));
    }

    #[test]
    fn test_generate_async_wrapper_has_no_block_on_dependency() {
        let func: ItemFn = parse_quote! {
            async fn demo(a: i32) -> i32 { a + 1 }
        };
        let aspect_info = AspectInfo::parse(parse_quote!(Logger)).unwrap();
        let tokens = generate_aspect_wrapper(&aspect_info, &func).to_string();

        assert!(!tokens.contains("ProceedingJoinPoint :: new"));
        assert!(!tokens.contains("__aspect . around (__pjp)"));
        assert!(!tokens.contains("tokio :: task :: block_in_place"));
    }

    #[test]
    fn test_generate_async_wrapper_for_impl_trait_has_no_axum_coupling() {
        let func: ItemFn = parse_quote! {
            async fn demo(a: i32) -> impl IntoResponse { a + 1 }
        };
        let aspect_info = AspectInfo::parse(parse_quote!(Logger)).unwrap();
        let tokens = generate_aspect_wrapper(&aspect_info, &func).to_string();

        assert!(tokens.contains("__aspect . before (& JoinPoint"));
        assert!(tokens.contains("__aspect . after (& __after_context"));
        assert!(!tokens.contains(":: axum :: response"));
        assert!(!tokens.contains("IntoResponse :: into_response"));
        assert!(!tokens.contains("block_in_place"));
    }

    #[test]
    fn test_async_wrappers_do_not_duplicate_after_calls() {
        let func: ItemFn = parse_quote! {
            async fn demo(a: i32) -> i32 { a + 1 }
        };
        let aspect_info = AspectInfo::parse(parse_quote!(Logger)).unwrap();

        let sync_tokens = generate_aspect_wrapper(&aspect_info, &func).to_string();
        let async_tokens = generate_async_aspect_wrapper(&aspect_info, &func).to_string();

        assert_eq!(sync_tokens.matches("__aspect . around (__pjp)").count(), 0);
        assert_eq!(sync_tokens.matches("__aspect . after_error (").count(), 0);
        assert_eq!(sync_tokens.matches("__aspect . after (& __after_context").count(), 1);
        assert_eq!(async_tokens.matches("__aspect . after (& __after_context").count(), 1);
        assert_eq!(async_tokens.matches("__aspect . after_error (").count(), 0);
    }

    #[test]
    fn test_generate_async_aspect_wrapper_keeps_lifecycle_for_non_response_impl_trait() {
        let func: ItemFn = parse_quote! {
            async fn demo(a: i32) -> impl core::fmt::Debug { a + 1 }
        };
        let aspect_info = AspectInfo::parse(parse_quote!(Logger)).unwrap();
        let tokens = generate_async_aspect_wrapper(&aspect_info, &func).to_string();

        assert!(!tokens.contains("AsyncProceedingJoinPoint :: new"));
        assert!(tokens.contains("__aspect . before (& AsyncJoinPoint"));
        assert!(tokens.contains("__aspect . after (& __after_context"));
        assert!(!tokens.contains("__aspect . around (__pjp) . await"));
    }
}
