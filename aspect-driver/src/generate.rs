//! Code generation for aspect weaving.
//!
//! This module generates the code that applies aspects to functions by
//! transforming the function body to include aspect calls.

use crate::r#match::{AdviceType, RegisteredAspect};
use crate::types::FunctionMetadata;

/// Generated code for a function with aspects applied.
#[derive(Debug, Clone)]
pub struct GeneratedFunction {
    /// Original function metadata
    pub original: FunctionMetadata,

    /// Generated function code (as Rust source)
    pub code: String,

    /// Applied aspects
    pub aspects: Vec<RegisteredAspect>,

    /// Whether the original function was renamed
    pub original_renamed: bool,
}

/// Code generator for aspect weaving.
pub struct AspectCodeGenerator {
    /// Counter for generating unique names
    id_counter: usize,
}

impl AspectCodeGenerator {
    /// Create a new code generator.
    pub fn new() -> Self {
        Self { id_counter: 0 }
    }

    /// Generate code for a function with aspects applied.
    ///
    /// # Strategy
    ///
    /// For a function with aspects, we:
    /// 1. Rename original function to `__aspect_original_<name>`
    /// 2. Create wrapper function with original name
    /// 3. Insert aspect calls (before/after/around)
    /// 4. Call original function from wrapper
    pub fn generate(
        &mut self,
        function: &FunctionMetadata,
        aspects: &[RegisteredAspect],
    ) -> GeneratedFunction {
        if aspects.is_empty() {
            // No aspects, return original
            return GeneratedFunction {
                original: function.clone(),
                code: format!("// Original function (no aspects)\n"),
                aspects: vec![],
                original_renamed: false,
            };
        }

        // Group aspects by type
        let before_aspects: Vec<_> = aspects
            .iter()
            .filter(|a| a.advice_type == AdviceType::Before)
            .collect();
        let after_aspects: Vec<_> = aspects
            .iter()
            .filter(|a| a.advice_type == AdviceType::After)
            .collect();
        let around_aspects: Vec<_> = aspects
            .iter()
            .filter(|a| a.advice_type == AdviceType::Around)
            .collect();

        // Generate code based on aspect types
        let code = if !around_aspects.is_empty() {
            self.generate_with_around(function, &before_aspects, &after_aspects, &around_aspects)
        } else {
            self.generate_with_before_after(function, &before_aspects, &after_aspects)
        };

        GeneratedFunction {
            original: function.clone(),
            code,
            aspects: aspects.to_vec(),
            original_renamed: true,
        }
    }

    /// Generate code with before/after aspects only.
    fn generate_with_before_after(
        &mut self,
        function: &FunctionMetadata,
        before: &[&RegisteredAspect],
        after: &[&RegisteredAspect],
    ) -> String {
        let mut code = String::new();

        // Original function (renamed)
        let original_name = self.original_function_name(function);
        code.push_str(&format!(
            "// Original function renamed\n\
             fn {original_name}(...) {{ ... }}\n\n"
        ));

        // Wrapper function
        let wrapper_name = self.simple_function_name(function);
        code.push_str(&format!("// Wrapper function with aspects\n"));
        code.push_str(&format!(
            "{} fn {}(",
            self.visibility_str(function),
            wrapper_name
        ));

        // Parameters (simplified)
        code.push_str("...) ");

        // Return type
        if !function.return_type.is_empty() && function.return_type != "()" {
            code.push_str(&format!("-> {} ", function.return_type));
        }

        code.push_str("{\n");

        // Create JoinPoint
        code.push_str("    let ctx = JoinPoint {\n");
        code.push_str(&format!("        function_name: \"{}\",\n", function.name));
        code.push_str(&format!(
            "        module_path: \"{}\",\n",
            function.module_path
        ));
        code.push_str("        location: Location { ... },\n");
        code.push_str("    };\n\n");

        // Before aspects
        for aspect in before {
            code.push_str(&format!("    // Before aspect: {}\n", aspect.aspect_name));
            code.push_str(&format!(
                "    {}::new().before(&ctx);\n\n",
                aspect.aspect_name
            ));
        }

        // Call original
        code.push_str(&format!("    let result = {original_name}(...);\n\n"));

        // After aspects
        for aspect in after {
            code.push_str(&format!("    // After aspect: {}\n", aspect.aspect_name));
            code.push_str(&format!(
                "    {}::new().after(&ctx, &result);\n\n",
                aspect.aspect_name
            ));
        }

        code.push_str("    result\n");
        code.push_str("}\n");

        code
    }

    /// Generate code with around aspects.
    fn generate_with_around(
        &mut self,
        function: &FunctionMetadata,
        before: &[&RegisteredAspect],
        after: &[&RegisteredAspect],
        around: &[&RegisteredAspect],
    ) -> String {
        let mut code = String::new();

        let original_name = self.original_function_name(function);
        let wrapper_name = self.simple_function_name(function);

        code.push_str(&format!(
            "// Original function renamed\n\
             fn {original_name}(...) {{ ... }}\n\n"
        ));

        code.push_str(&format!("// Wrapper with around advice\n"));
        code.push_str(&format!(
            "{} fn {}(",
            self.visibility_str(function),
            wrapper_name
        ));
        code.push_str("...) ");

        if !function.return_type.is_empty() && function.return_type != "()" {
            code.push_str(&format!(
                "-> Result<{}, AspectError> ",
                function.return_type
            ));
        } else {
            code.push_str("-> Result<(), AspectError> ");
        }

        code.push_str("{\n");

        // JoinPoint
        code.push_str("    let ctx = JoinPoint { ... };\n\n");

        // Before aspects
        for aspect in before {
            code.push_str(&format!(
                "    {}::new().before(&ctx);\n",
                aspect.aspect_name
            ));
        }

        // Create ProceedingJoinPoint
        code.push_str("\n    let pjp = ProceedingJoinPoint::new(\n");
        code.push_str(&format!(
            "        || Ok(Box::new({original_name}(...)) as Box<dyn Any>),\n"
        ));
        code.push_str("        ctx.clone()\n");
        code.push_str("    );\n\n");

        // Chain around aspects (innermost to outermost)
        code.push_str("    let mut pjp = pjp;\n");
        for aspect in around.iter().rev() {
            code.push_str(&format!(
                "    let pjp = {}::new().around(pjp)?;\n",
                aspect.aspect_name
            ));
        }

        code.push_str("\n    let result = pjp.proceed()?;\n\n");

        // After aspects
        for aspect in after {
            code.push_str(&format!(
                "    {}::new().after(&ctx, &result);\n",
                aspect.aspect_name
            ));
        }

        code.push_str("    Ok(result)\n");
        code.push_str("}\n");

        code
    }

    /// Get the original function name (with prefix).
    fn original_function_name(&self, function: &FunctionMetadata) -> String {
        let simple_name = self.simple_function_name(function);
        format!("__aspect_original_{}", simple_name)
    }

    /// Get the simple function name (last component).
    fn simple_function_name(&self, function: &FunctionMetadata) -> String {
        function
            .name
            .rsplit("::")
            .next()
            .unwrap_or(&function.name)
            .to_string()
    }

    /// Get visibility as string.
    fn visibility_str(&self, function: &FunctionMetadata) -> &'static str {
        match function.visibility {
            crate::types::Visibility::Public => "pub",
            crate::types::Visibility::Crate => "pub(crate)",
            crate::types::Visibility::Restricted => "pub(in ...)",
            crate::types::Visibility::Private => "",
        }
    }

    /// Generate a unique identifier.
    fn unique_id(&mut self) -> usize {
        let id = self.id_counter;
        self.id_counter += 1;
        id
    }
}

impl Default for AspectCodeGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// Transform a function's MIR/HIR to apply aspects.
///
/// # Full Implementation (requires rustc)
///
/// ```ignore
/// use rustc_middle::mir::{Body, BasicBlock, Statement};
/// use rustc_middle::ty::TyCtxt;
///
/// fn transform_mir<'tcx>(
///     tcx: TyCtxt<'tcx>,
///     body: &mut Body<'tcx>,
///     aspects: &[RegisteredAspect],
/// ) {
///     // Get entry basic block
///     let entry_block = &mut body.basic_blocks_mut()[BasicBlock::from_u32(0)];
///
///     // Insert before advice
///     for aspect in aspects.iter().filter(|a| a.advice_type == AdviceType::Before) {
///         let before_call = generate_before_call(tcx, aspect);
///         entry_block.statements.insert(0, before_call);
///     }
///
///     // Find return statements
///     for block in body.basic_blocks_mut() {
///         if let Some(terminator) = &mut block.terminator {
///             if is_return(terminator) {
///                 // Insert after advice before return
///                 for aspect in aspects.iter().filter(|a| a.advice_type == AdviceType::After) {
///                     let after_call = generate_after_call(tcx, aspect);
///                     block.statements.push(after_call);
///                 }
///             }
///         }
///     }
/// }
/// ```
pub fn transform_mir() -> String {
    // Placeholder - would modify MIR in place
    String::from("// MIR transformation requires rustc integration")
}

/// Generate aspect initialization code.
pub fn generate_aspect_init(aspect: &RegisteredAspect) -> String {
    format!("let aspect = {}::new();", aspect.aspect_name)
}

/// Generate JoinPoint creation code.
pub fn generate_joinpoint(function: &FunctionMetadata) -> String {
    format!(
        "let ctx = JoinPoint {{\n\
            function_name: \"{}\",\n\
            module_path: \"{}\",\n\
            location: Location {{ file: \"{}\", line: {} }}\n\
        }};",
        function.name, function.module_path, function.location.file, function.location.line
    )
}

/// Generate before advice call.
pub fn generate_before_call(aspect: &RegisteredAspect) -> String {
    format!("{}::new().before(&ctx);", aspect.aspect_name)
}

/// Generate after advice call.
pub fn generate_after_call(aspect: &RegisteredAspect) -> String {
    format!("{}::new().after(&ctx, &result);", aspect.aspect_name)
}

/// Generate around advice wrapper.
pub fn generate_around_wrapper(aspects: &[RegisteredAspect], original_call: &str) -> String {
    let mut code = String::new();

    code.push_str("let pjp = ProceedingJoinPoint::new(\n");
    code.push_str(&format!(
        "    || Ok(Box::new({}) as Box<dyn Any>),\n",
        original_call
    ));
    code.push_str("    ctx.clone()\n");
    code.push_str(");\n");

    for aspect in aspects {
        code.push_str(&format!(
            "let pjp = {}::new().around(pjp)?;\n",
            aspect.aspect_name
        ));
    }

    code.push_str("let result = pjp.proceed()?;\n");

    code
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{SourceLocation, Visibility};

    fn sample_function() -> FunctionMetadata {
        FunctionMetadata {
            name: "crate::api::fetch_user".to_string(),
            simple_name: "fetch_user".to_string(),
            module_path: "crate::api".to_string(),
            visibility: Visibility::Public,
            is_async: false,
            generics: vec![],
            return_type: "User".to_string(),
            location: SourceLocation {
                file: "src/api.rs".to_string(),
                line: 42,
                column: 1,
            },
        }
    }

    fn sample_aspect(name: &str, advice: AdviceType) -> RegisteredAspect {
        RegisteredAspect {
            aspect_name: name.to_string(),
            pointcut: "execution(pub fn *(..))".to_string(),
            advice_type: advice,
            priority: 0,
        }
    }

    #[test]
    fn test_generate_no_aspects() {
        let mut gen = AspectCodeGenerator::new();
        let func = sample_function();
        let result = gen.generate(&func, &[]);

        assert!(!result.original_renamed);
        assert_eq!(result.aspects.len(), 0);
    }

    #[test]
    fn test_generate_with_before() {
        let mut gen = AspectCodeGenerator::new();
        let func = sample_function();
        let aspects = vec![sample_aspect("Logger", AdviceType::Before)];

        let result = gen.generate(&func, &aspects);

        assert!(result.original_renamed);
        assert!(result.code.contains("Logger::new().before(&ctx)"));
        assert!(result.code.contains("__aspect_original_fetch_user"));
    }

    #[test]
    fn test_generate_with_after() {
        let mut gen = AspectCodeGenerator::new();
        let func = sample_function();
        let aspects = vec![sample_aspect("Timer", AdviceType::After)];

        let result = gen.generate(&func, &aspects);

        assert!(result.code.contains("Timer::new().after(&ctx, &result)"));
    }

    #[test]
    fn test_generate_with_around() {
        let mut gen = AspectCodeGenerator::new();
        let func = sample_function();
        let aspects = vec![sample_aspect("Cache", AdviceType::Around)];

        let result = gen.generate(&func, &aspects);

        assert!(result.code.contains("ProceedingJoinPoint::new"));
        assert!(result.code.contains("Cache::new().around(pjp)"));
    }

    #[test]
    fn test_generate_multiple_aspects() {
        let mut gen = AspectCodeGenerator::new();
        let func = sample_function();
        let aspects = vec![
            sample_aspect("Logger", AdviceType::Before),
            sample_aspect("Timer", AdviceType::After),
        ];

        let result = gen.generate(&func, &aspects);

        assert!(result.code.contains("Logger::new().before(&ctx)"));
        assert!(result.code.contains("Timer::new().after(&ctx, &result)"));
    }

    #[test]
    fn test_original_function_name() {
        let gen = AspectCodeGenerator::new();
        let func = sample_function();
        let name = gen.original_function_name(&func);

        assert_eq!(name, "__aspect_original_fetch_user");
    }

    #[test]
    fn test_simple_function_name() {
        let gen = AspectCodeGenerator::new();
        let func = sample_function();
        let name = gen.simple_function_name(&func);

        assert_eq!(name, "fetch_user");
    }

    #[test]
    fn test_visibility_str() {
        let gen = AspectCodeGenerator::new();
        let func = sample_function();
        assert_eq!(gen.visibility_str(&func), "pub");

        let private_func = FunctionMetadata {
            visibility: Visibility::Private,
            ..func
        };
        assert_eq!(gen.visibility_str(&private_func), "");
    }

    #[test]
    fn test_generate_joinpoint() {
        let func = sample_function();
        let code = generate_joinpoint(&func);

        assert!(code.contains("function_name: \"crate::api::fetch_user\""));
        assert!(code.contains("module_path: \"crate::api\""));
    }

    #[test]
    fn test_generate_aspect_init() {
        let aspect = sample_aspect("Logger", AdviceType::Before);
        let code = generate_aspect_init(&aspect);

        assert_eq!(code, "let aspect = Logger::new();");
    }

    #[test]
    fn test_generate_before_call() {
        let aspect = sample_aspect("Logger", AdviceType::Before);
        let code = generate_before_call(&aspect);

        assert_eq!(code, "Logger::new().before(&ctx);");
    }

    #[test]
    fn test_generate_after_call() {
        let aspect = sample_aspect("Timer", AdviceType::After);
        let code = generate_after_call(&aspect);

        assert_eq!(code, "Timer::new().after(&ctx, &result);");
    }

    #[test]
    fn test_generate_around_wrapper() {
        let aspects = vec![sample_aspect("Cache", AdviceType::Around)];
        let code = generate_around_wrapper(&aspects, "original_function()");

        assert!(code.contains("ProceedingJoinPoint::new"));
        assert!(code.contains("original_function()"));
        assert!(code.contains("Cache::new().around(pjp)"));
    }
}
