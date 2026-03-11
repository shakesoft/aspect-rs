//! Global aspect registry for managing aspect-pointcut bindings.
//!
//! The registry allows aspects to be registered with pointcut patterns,
//! and then automatically applied to matching functions at runtime.

use aspect_core::pointcut::{FunctionInfo, Matcher, Pointcut};
use aspect_core::{Aspect, ProceedingJoinPoint};
use once_cell::sync::Lazy;
use std::sync::{Arc, RwLock};

/// A registered aspect with its associated pointcut and metadata.
#[derive(Clone)]
pub struct RegisteredAspect {
    /// The aspect instance
    pub aspect: Arc<dyn Aspect>,

    /// The pointcut pattern this aspect matches
    pub pointcut: Pointcut,

    /// Execution order (lower values run first/outermost)
    pub order: i32,

    /// Optional name for debugging
    pub name: Option<String>,
}

/// Global aspect registry for managing aspect-pointcut bindings.
///
/// The registry is thread-safe and can be accessed from anywhere in the program.
/// Aspects are matched against functions using their pointcut patterns.
pub struct AspectRegistry {
    aspects: RwLock<Vec<RegisteredAspect>>,
}

impl AspectRegistry {
    /// Create a new empty registry.
    fn new() -> Self {
        Self {
            aspects: RwLock::new(Vec::new()),
        }
    }

    /// Register an aspect with a pointcut pattern.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use aspect_runtime::registry::global_registry;
    /// use aspect_core::pointcut::Pointcut;
    /// use std::sync::Arc;
    ///
    /// // Register an aspect (my_aspect would be an actual Aspect implementation)
    /// let pointcut = Pointcut::parse("execution(pub fn *(..))").unwrap();
    /// // global_registry().register(Arc::new(my_aspect), pointcut, 0, Some("my_aspect".into()));
    /// ```
    pub fn register(
        &self,
        aspect: Arc<dyn Aspect>,
        pointcut: Pointcut,
        order: i32,
        name: Option<String>,
    ) {
        let mut aspects = self.aspects.write().unwrap();
        aspects.push(RegisteredAspect {
            aspect,
            pointcut,
            order,
            name,
        });

        // Sort by order (lower values first)
        aspects.sort_by_key(|a| a.order);
    }

    /// Find all aspects that match the given function.
    ///
    /// Returns aspects in execution order (sorted by `order` field).
    pub fn find_matching(&self, function: &FunctionInfo) -> Vec<RegisteredAspect> {
        let aspects = self.aspects.read().unwrap();
        aspects
            .iter()
            .filter(|registered| registered.pointcut.matches(function))
            .cloned()
            .collect()
    }

    /// Apply all matching aspects to a function execution.
    ///
    /// This creates a chain of aspects, with lower-order aspects wrapping higher-order ones.
    pub fn apply_aspects(
        &self,
        function: &FunctionInfo,
        mut pjp: ProceedingJoinPoint,
    ) -> Result<Box<dyn std::any::Any>, aspect_core::AspectError> {
        let matching = self.find_matching(function);

        if matching.is_empty() {
            // No aspects match, just proceed
            return pjp.proceed();
        }

        // Apply aspects in order (outermost first)
        // Each aspect wraps the previous one
        for registered in matching.iter().rev() {
            let aspect = Arc::clone(&registered.aspect);
            let inner_pjp = pjp;

            // Create a new ProceedingJoinPoint that wraps the aspect application
            pjp = ProceedingJoinPoint::new(
                move || aspect.around(inner_pjp),
                function_info_to_joinpoint(function),
            );
        }

        pjp.proceed()
    }

    /// Get the number of registered aspects.
    pub fn count(&self) -> usize {
        self.aspects.read().unwrap().len()
    }

    /// Clear all registered aspects (useful for testing).
    pub fn clear(&self) {
        self.aspects.write().unwrap().clear();
    }
}

/// Global aspect registry instance.
///
/// This is a singleton that can be accessed from anywhere in the program.
pub static GLOBAL_REGISTRY: Lazy<AspectRegistry> = Lazy::new(AspectRegistry::new);

/// Get a reference to the global aspect registry.
///
/// # Example
///
/// ```rust
/// use aspect_runtime::registry::global_registry;
///
/// let registry = global_registry();
/// println!("Registered aspects: {}", registry.count());
/// ```
pub fn global_registry() -> &'static AspectRegistry {
    &GLOBAL_REGISTRY
}

/// Helper to convert FunctionInfo to JoinPoint
fn function_info_to_joinpoint(info: &FunctionInfo) -> aspect_core::JoinPoint {
    aspect_core::JoinPoint {
        function_name: Box::leak(info.name.clone().into_boxed_str()),
        module_path: Box::leak(info.module_path.clone().into_boxed_str()),
        location: aspect_core::Location {
            file: "unknown",
            line: 0,
        },
        args: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aspect_core::{Aspect, JoinPoint};
    use std::any::Any;
    use std::sync::{Arc, Mutex};

    #[derive(Clone)]
    struct TestAspect {
        name: String,
        called: Arc<Mutex<Vec<String>>>,
    }

    impl Aspect for TestAspect {
        fn before(&self, ctx: &JoinPoint) {
            self.called
                .lock()
                .unwrap()
                .push(format!("{}:before:{}", self.name, ctx.function_name));
        }

        fn after(&self, ctx: &JoinPoint, _result: &dyn Any) {
            self.called
                .lock()
                .unwrap()
                .push(format!("{}:after:{}", self.name, ctx.function_name));
        }
    }

    #[test]
    fn test_registry_register_and_find() {
        let registry = AspectRegistry::new();

        let calls = Arc::new(Mutex::new(Vec::new()));
        let aspect = Arc::new(TestAspect {
            name: "test".to_string(),
            called: calls.clone(),
        });

        let pointcut = Pointcut::parse("execution(pub fn *(..))").unwrap();
        registry.register(aspect, pointcut, 0, Some("test".into()));

        assert_eq!(registry.count(), 1);

        let function = FunctionInfo {
            name: "test_func".to_string(),
            module_path: "test::module".to_string(),
            visibility: "pub".to_string(),
            return_type: None,
        };

        let matching = registry.find_matching(&function);
        assert_eq!(matching.len(), 1);
        assert_eq!(matching[0].name.as_deref(), Some("test"));
    }

    #[test]
    fn test_aspect_ordering() {
        let registry = AspectRegistry::new();

        let calls1 = Arc::new(Mutex::new(Vec::new()));
        let aspect1 = Arc::new(TestAspect {
            name: "first".to_string(),
            called: calls1.clone(),
        });

        let calls2 = Arc::new(Mutex::new(Vec::new()));
        let aspect2 = Arc::new(TestAspect {
            name: "second".to_string(),
            called: calls2.clone(),
        });

        let pointcut = Pointcut::parse("execution(pub fn *(..))").unwrap();

        // Register in reverse order to test sorting
        registry.register(aspect2, pointcut.clone(), 20, Some("second".into()));
        registry.register(aspect1, pointcut, 10, Some("first".into()));

        let function = FunctionInfo {
            name: "test_func".to_string(),
            module_path: "test::module".to_string(),
            visibility: "pub".to_string(),
            return_type: None,
        };

        let matching = registry.find_matching(&function);
        assert_eq!(matching.len(), 2);
        assert_eq!(matching[0].name.as_deref(), Some("first"));
        assert_eq!(matching[1].name.as_deref(), Some("second"));
    }

    #[test]
    fn test_pointcut_matching() {
        let registry = AspectRegistry::new();

        let calls = Arc::new(Mutex::new(Vec::new()));
        let aspect = Arc::new(TestAspect {
            name: "api".to_string(),
            called: calls.clone(),
        });

        let pointcut = Pointcut::parse("execution(pub fn *(..)) && within(crate::api)").unwrap();
        registry.register(aspect, pointcut, 0, Some("api".into()));

        // Should match
        let func1 = FunctionInfo {
            name: "save_user".to_string(),
            module_path: "crate::api".to_string(),
            visibility: "pub".to_string(),
            return_type: None,
        };
        assert_eq!(registry.find_matching(&func1).len(), 1);

        // Should not match (wrong module)
        let func2 = FunctionInfo {
            name: "save_user".to_string(),
            module_path: "crate::internal".to_string(),
            visibility: "pub".to_string(),
            return_type: None,
        };
        assert_eq!(registry.find_matching(&func2).len(), 0);

        // Should not match (not public)
        let func3 = FunctionInfo {
            name: "save_user".to_string(),
            module_path: "crate::api".to_string(),
            visibility: "".to_string(),
            return_type: None,
        };
        assert_eq!(registry.find_matching(&func3).len(), 0);
    }
}
