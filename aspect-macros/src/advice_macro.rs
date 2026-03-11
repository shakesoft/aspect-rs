//! Implementation of the #[advice] attribute macro.
//!
//! The #[advice] macro allows declarative aspect application using pointcut expressions.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse::Parse, parse::ParseStream, Error, Expr, ItemFn, LitStr, Result, Token};

/// Parsed attributes for the #[advice] macro.
#[derive(Debug)]
pub struct AdviceArgs {
    /// Pointcut expression string
    pub pointcut: String,

    /// Advice type: "before", "after", "after_error", or "around"
    pub advice_type: Option<String>,

    /// Execution order (lower runs first)
    pub order: i32,
}

impl Parse for AdviceArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut pointcut = None;
        let mut advice_type = None;
        let mut order = 0;

        // Parse key-value pairs: pointcut = "...", advice = "...", order = 10
        while !input.is_empty() {
            let key: syn::Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            match key.to_string().as_str() {
                "pointcut" => {
                    let value: LitStr = input.parse()?;
                    pointcut = Some(value.value());
                }
                "advice" => {
                    let value: LitStr = input.parse()?;
                    advice_type = Some(value.value());
                }
                "order" => {
                    let value: Expr = input.parse()?;
                    // Try to extract integer literal
                    if let Expr::Lit(expr_lit) = value {
                        if let syn::Lit::Int(lit_int) = expr_lit.lit {
                            order = lit_int.base10_parse()?;
                        }
                    }
                }
                _ => {
                    return Err(Error::new(
                        key.span(),
                        format!("Unknown attribute key: {}", key),
                    ))
                }
            }

            // Parse optional comma
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        let pointcut = pointcut
            .ok_or_else(|| Error::new(input.span(), "Missing required attribute: pointcut"))?;

        Ok(AdviceArgs {
            pointcut,
            advice_type,
            order,
        })
    }
}

/// Transform a function with the #[advice] attribute.
///
/// Generates code that:
/// 1. Wraps the function in an Aspect implementation
/// 2. Registers it with the global aspect registry
/// 3. Associates it with the given pointcut pattern
pub fn transform(args: AdviceArgs, func: ItemFn) -> Result<TokenStream> {
    let func_name = &func.sig.ident;
    let aspect_struct_name = quote::format_ident!("{}Aspect", func_name);
    let registrar_name = quote::format_ident!("__register_{}", func_name);

    let pointcut_str = &args.pointcut;
    let order = args.order;

    // Determine which advice method to implement based on advice_type
    let advice_impl = match args.advice_type.as_deref() {
        Some("before") => generate_before_impl(&func)?,
        Some("after") => generate_after_impl(&func)?,
        Some("after_error") => generate_after_error_impl(&func)?,
        Some("around") | None => generate_around_impl(&func)?,
        Some(other) => {
            return Err(Error::new_spanned(
                &func.sig.ident,
                format!(
                    "Invalid advice type: {}. Must be one of: before, after, after_error, around",
                    other
                ),
            ))
        }
    };

    // Generate the aspect struct and registration code
    let output = quote! {
        // Original function (kept for potential direct calls)
        #func

        // Generated aspect wrapper
        #[derive(Clone, Copy)]
        struct #aspect_struct_name;

        impl aspect_core::Aspect for #aspect_struct_name {
            #advice_impl
        }

        // Registration function using once_cell::Lazy
        #[allow(non_upper_case_globals)]
        static #registrar_name: aspect_runtime::once_cell::sync::Lazy<()> =
            aspect_runtime::once_cell::sync::Lazy::new(|| {
                let pointcut = aspect_core::pointcut::Pointcut::parse(#pointcut_str)
                    .expect(&format!("Invalid pointcut expression: {}", #pointcut_str));

                aspect_runtime::global_registry().register(
                    std::sync::Arc::new(#aspect_struct_name),
                    pointcut,
                    #order,
                    Some(stringify!(#func_name).to_string()),
                );
            });

        // Force registration by referencing the static
        const _: () = {
            let _ = &#registrar_name as &aspect_runtime::once_cell::sync::Lazy<()>;
        };
    };

    Ok(output)
}

/// Generate `before` advice implementation.
fn generate_before_impl(func: &ItemFn) -> Result<TokenStream> {
    let func_name = &func.sig.ident;

    Ok(quote! {
        fn before(&self, ctx: &aspect_core::JoinPoint) {
            #func_name(ctx);
        }
    })
}

/// Generate `after` advice implementation.
fn generate_after_impl(func: &ItemFn) -> Result<TokenStream> {
    let func_name = &func.sig.ident;

    Ok(quote! {
        fn after(&self, ctx: &aspect_core::JoinPoint, result: &dyn std::any::Any) {
            #func_name(ctx, result);
        }
    })
}

/// Generate `after_error` advice implementation.
fn generate_after_error_impl(func: &ItemFn) -> Result<TokenStream> {
    let func_name = &func.sig.ident;

    Ok(quote! {
        fn after_error(&self, ctx: &aspect_core::JoinPoint, error: &aspect_core::AspectError) {
            #func_name(ctx, error);
        }
    })
}

/// Generate `around` advice implementation.
fn generate_around_impl(func: &ItemFn) -> Result<TokenStream> {
    let func_name = &func.sig.ident;

    Ok(quote! {
        fn around(
            &self,
            pjp: aspect_core::ProceedingJoinPoint
        ) -> Result<Box<dyn std::any::Any>, aspect_core::AspectError> {
            #func_name(pjp)
        }
    })
}
