//! Validation aspect for pre/post condition checking.

use aspect_core::{Aspect, AspectError, JoinPoint, ProceedingJoinPoint};
use std::any::Any;
use std::sync::Arc;

/// Validation rule trait.
///
/// Implement this trait to create custom validation rules that can be
/// composed and applied to functions.
pub trait ValidationRule: Send + Sync {
    /// Validate the input.
    ///
    /// Returns `Ok(())` if validation passes, or `Err(message)` if it fails.
    fn validate(&self, ctx: &JoinPoint) -> Result<(), String>;

    /// Get a description of this validation rule.
    fn description(&self) -> &str {
        "validation rule"
    }
}

/// Validation aspect for enforcing constraints.
///
/// Allows composing multiple validation rules that are checked before
/// function execution.
///
/// # Example
///
/// ```rust,ignore
/// use aspect_std::{ValidationAspect, ValidationRule};
/// use aspect_macros::aspect;
///
/// struct AgeValidator;
/// impl ValidationRule for AgeValidator {
///     fn validate(&self, ctx: &JoinPoint) -> Result<(), String> {
///         // Validation logic
///         Ok(())
///     }
/// }
///
/// let validator = ValidationAspect::new()
///     .add_rule(Box::new(AgeValidator));
///
/// #[aspect(validator)]
/// fn set_age(age: i32) -> Result<(), String> {
///     Ok(())
/// }
/// ```
pub struct ValidationAspect {
    rules: Vec<Box<dyn ValidationRule>>,
}

impl ValidationAspect {
    /// Create a new validation aspect.
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    /// Add a validation rule.
    pub fn add_rule(mut self, rule: Box<dyn ValidationRule>) -> Self {
        self.rules.push(rule);
        self
    }

    /// Run all validation rules.
    fn validate(&self, ctx: &JoinPoint) -> Result<(), AspectError> {
        for rule in self.rules.iter() {
            if let Err(msg) = rule.validate(ctx) {
                return Err(AspectError::execution(format!(
                    "Validation failed for {}: {}",
                    ctx.function_name, msg
                )));
            }
        }
        Ok(())
    }
}

impl Default for ValidationAspect {
    fn default() -> Self {
        Self::new()
    }
}

impl Aspect for ValidationAspect {
    fn around(&self, pjp: ProceedingJoinPoint) -> Result<Box<dyn Any>, AspectError> {
        // Validate before execution
        self.validate(pjp.context())?;

        // Execute the function
        // Note: After-validation is not supported in this simplified version
        // as it would require cloning the context
        pjp.proceed()
    }
}

// Common validation rules

/// Validates that a value is not empty.
pub struct NotEmptyValidator {
    field_name: String,
    getter: Arc<dyn Fn(&JoinPoint) -> Option<String> + Send + Sync>,
}

impl NotEmptyValidator {
    /// Create a new not-empty validator.
    ///
    /// # Arguments
    /// * `field_name` - Name of the field being validated
    /// * `getter` - Function to extract the value from JoinPoint
    pub fn new<F>(field_name: &str, getter: F) -> Self
    where
        F: Fn(&JoinPoint) -> Option<String> + Send + Sync + 'static,
    {
        Self {
            field_name: field_name.to_string(),
            getter: Arc::new(getter),
        }
    }
}

impl ValidationRule for NotEmptyValidator {
    fn validate(&self, ctx: &JoinPoint) -> Result<(), String> {
        if let Some(value) = (self.getter)(ctx) {
            if value.is_empty() {
                return Err(format!("{} cannot be empty", self.field_name));
            }
        }
        Ok(())
    }

    fn description(&self) -> &str {
        "not empty"
    }
}

/// Validates that a numeric value is within a range.
pub struct RangeValidator {
    field_name: String,
    min: i64,
    max: i64,
    getter: Arc<dyn Fn(&JoinPoint) -> Option<i64> + Send + Sync>,
}

impl RangeValidator {
    /// Create a new range validator.
    ///
    /// # Arguments
    /// * `field_name` - Name of the field being validated
    /// * `min` - Minimum allowed value (inclusive)
    /// * `max` - Maximum allowed value (inclusive)
    /// * `getter` - Function to extract the value from JoinPoint
    pub fn new<F>(field_name: &str, min: i64, max: i64, getter: F) -> Self
    where
        F: Fn(&JoinPoint) -> Option<i64> + Send + Sync + 'static,
    {
        Self {
            field_name: field_name.to_string(),
            min,
            max,
            getter: Arc::new(getter),
        }
    }
}

impl ValidationRule for RangeValidator {
    fn validate(&self, ctx: &JoinPoint) -> Result<(), String> {
        if let Some(value) = (self.getter)(ctx) {
            if value < self.min || value > self.max {
                return Err(format!(
                    "{} must be between {} and {}, got {}",
                    self.field_name, self.min, self.max, value
                ));
            }
        }
        Ok(())
    }

    fn description(&self) -> &str {
        "range check"
    }
}

/// Custom validation rule using a closure.
pub struct CustomValidator {
    description: String,
    validator: Arc<dyn Fn(&JoinPoint) -> Result<(), String> + Send + Sync>,
}

impl CustomValidator {
    /// Create a custom validator from a closure.
    ///
    /// # Arguments
    /// * `description` - Description of this validation
    /// * `validator` - Validation function
    pub fn new<F>(description: &str, validator: F) -> Self
    where
        F: Fn(&JoinPoint) -> Result<(), String> + Send + Sync + 'static,
    {
        Self {
            description: description.to_string(),
            validator: Arc::new(validator),
        }
    }
}

impl ValidationRule for CustomValidator {
    fn validate(&self, ctx: &JoinPoint) -> Result<(), String> {
        (self.validator)(ctx)
    }

    fn description(&self) -> &str {
        &self.description
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_aspect_creation() {
        let validator = ValidationAspect::new();
        assert_eq!(validator.rules.len(), 0);
    }

    #[test]
    fn test_custom_validator() {
        let validator = CustomValidator::new("test", |_ctx| Ok(()));
        let ctx = JoinPoint {
            function_name: "test",
            module_path: "test",
            location: aspect_core::Location {
                file: "test.rs",
                line: 1,
            },
            args: vec![],
        };

        assert!(validator.validate(&ctx).is_ok());
    }

    #[test]
    fn test_custom_validator_failure() {
        let validator = CustomValidator::new("test", |_ctx| Err("validation failed".to_string()));
        let ctx = JoinPoint {
            function_name: "test",
            module_path: "test",
            location: aspect_core::Location {
                file: "test.rs",
                line: 1,
            },
            args: vec![],
        };

        assert!(validator.validate(&ctx).is_err());
    }

    #[test]
    fn test_not_empty_validator() {
        let validator = NotEmptyValidator::new("username", |_ctx| Some("alice".to_string()));
        let ctx = JoinPoint {
            function_name: "test",
            module_path: "test",
            location: aspect_core::Location {
                file: "test.rs",
                line: 1,
            },
            args: vec![],
        };

        assert!(validator.validate(&ctx).is_ok());
    }

    #[test]
    fn test_not_empty_validator_failure() {
        let validator = NotEmptyValidator::new("username", |_ctx| Some("".to_string()));
        let ctx = JoinPoint {
            function_name: "test",
            module_path: "test",
            location: aspect_core::Location {
                file: "test.rs",
                line: 1,
            },
            args: vec![],
        };

        let result = validator.validate(&ctx);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cannot be empty"));
    }

    #[test]
    fn test_range_validator() {
        let validator = RangeValidator::new("age", 0, 120, |_ctx| Some(25));
        let ctx = JoinPoint {
            function_name: "test",
            module_path: "test",
            location: aspect_core::Location {
                file: "test.rs",
                line: 1,
            },
            args: vec![],
        };

        assert!(validator.validate(&ctx).is_ok());
    }

    #[test]
    fn test_range_validator_failure() {
        let validator = RangeValidator::new("age", 0, 120, |_ctx| Some(150));
        let ctx = JoinPoint {
            function_name: "test",
            module_path: "test",
            location: aspect_core::Location {
                file: "test.rs",
                line: 1,
            },
            args: vec![],
        };

        let result = validator.validate(&ctx);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("must be between"));
    }
}
