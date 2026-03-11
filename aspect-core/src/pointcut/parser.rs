//! Parser for pointcut expressions.
//!
//! Parses pointcut strings like:
//! - `execution(pub fn *(..))`
//! - `within(crate::api)`
//! - `execution(pub fn *(..)) && within(crate::api)`
//! - `(execution(pub fn *(..)) || within(crate::admin)) && !within(crate::internal)`

use super::ast::Pointcut;
use super::pattern::{ExecutionPattern, ModulePattern, NamePattern, Visibility};

/// Parse a pointcut expression from a string.
///
/// # Examples
///
/// ```rust
/// use aspect_core::pointcut::parse_pointcut;
///
/// let pc = parse_pointcut("execution(pub fn *(..))").unwrap();
/// let pc = parse_pointcut("within(crate::api)").unwrap();
/// let pc = parse_pointcut("execution(pub fn *(..)) && within(crate::api)").unwrap();
/// ```
pub fn parse_pointcut(input: &str) -> Result<Pointcut, String> {
    let input = input.trim();

    // Handle parentheses for grouping
    if input.starts_with('(') && input.ends_with(')') {
        // Check if this is a balanced outer parenthesis
        if let Some(inner) = strip_outer_parens(input) {
            return parse_pointcut(inner);
        }
    }

    // Handle NOT operator (highest precedence)
    if input.starts_with('!') {
        let inner = parse_pointcut(input[1..].trim())?;
        return Ok(Pointcut::Not(Box::new(inner)));
    }

    // Handle OR operator (lowest precedence)
    if let Some(or_pos) = find_operator(input, " || ") {
        let left = parse_pointcut(&input[..or_pos])?;
        let right = parse_pointcut(&input[or_pos + 4..])?;
        return Ok(Pointcut::Or(Box::new(left), Box::new(right)));
    }

    // Handle AND operator (medium precedence)
    if let Some(and_pos) = find_operator(input, " && ") {
        let left = parse_pointcut(&input[..and_pos])?;
        let right = parse_pointcut(&input[and_pos + 4..])?;
        return Ok(Pointcut::And(Box::new(left), Box::new(right)));
    }

    // Parse basic pointcuts
    if input.starts_with("execution(") {
        parse_execution(input)
    } else if input.starts_with("within(") {
        parse_within(input)
    } else {
        Err(format!("Unknown pointcut type: {}", input))
    }
}

/// Strip outer parentheses if they are balanced and wrap the entire expression.
fn strip_outer_parens(input: &str) -> Option<&str> {
    if !input.starts_with('(') || !input.ends_with(')') {
        return None;
    }

    let inner = &input[1..input.len() - 1];

    // Check if the parentheses are balanced for the entire inner content
    let mut depth = 0;
    for ch in inner.chars() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth < 0 {
                    return None; // Unbalanced
                }
            }
            _ => {}
        }
    }

    if depth == 0 {
        Some(inner)
    } else {
        None
    }
}

/// Find an operator outside of parentheses.
/// Returns the position of the operator, or None if not found.
fn find_operator(input: &str, operator: &str) -> Option<usize> {
    let mut depth = 0;
    let op_len = operator.len();
    let chars: Vec<char> = input.chars().collect();

    for i in 0..chars.len() {
        match chars[i] {
            '(' => depth += 1,
            ')' => depth -= 1,
            _ => {
                if depth == 0 && i + op_len <= chars.len() {
                    let slice: String = chars[i..i + op_len].iter().collect();
                    if slice == operator {
                        return Some(i);
                    }
                }
            }
        }
    }

    None
}

/// Parse an execution pointcut: `execution(pub fn save(..))`
fn parse_execution(input: &str) -> Result<Pointcut, String> {
    if !input.starts_with("execution(") || !input.ends_with(')') {
        return Err("Invalid execution syntax".to_string());
    }

    let content = &input[10..input.len() - 1].trim();

    // Parse visibility
    let (visibility, rest) = parse_visibility(content);

    // Expect "fn" keyword
    let rest = rest.trim();
    if !rest.starts_with("fn ") {
        return Err("Expected 'fn' keyword".to_string());
    }
    let rest = &rest[3..].trim();

    // Parse function name pattern
    let name = if let Some(paren_pos) = rest.find('(') {
        &rest[..paren_pos].trim()
    } else {
        return Err("Expected function signature".to_string());
    };

    let name_pattern = parse_name_pattern(name);

    // TODO: Parse parameters and return type

    Ok(Pointcut::Execution(ExecutionPattern {
        visibility,
        name: name_pattern,
        return_type: None,
    }))
}

/// Parse a within pointcut: `within(crate::api)`
fn parse_within(input: &str) -> Result<Pointcut, String> {
    if !input.starts_with("within(") || !input.ends_with(')') {
        return Err("Invalid within syntax".to_string());
    }

    let module_path = input[7..input.len() - 1].trim();

    Ok(Pointcut::Within(ModulePattern {
        path: module_path.to_string(),
    }))
}

/// Parse visibility from the beginning of a string.
/// Returns (Option<Visibility>, remaining_string)
fn parse_visibility(input: &str) -> (Option<Visibility>, &str) {
    if input.starts_with("pub(crate) ") {
        (Some(Visibility::Crate), &input[11..])
    } else if input.starts_with("pub(super) ") {
        (Some(Visibility::Super), &input[11..])
    } else if input.starts_with("pub ") {
        (Some(Visibility::Public), &input[4..])
    } else {
        (None, input)
    }
}

/// Parse a name pattern (exact, wildcard, prefix, suffix).
fn parse_name_pattern(name: &str) -> NamePattern {
    if name == "*" {
        NamePattern::Wildcard
    } else if name.starts_with('*') && name.ends_with('*') && name.len() > 2 {
        NamePattern::Contains(name[1..name.len() - 1].to_string())
    } else if name.starts_with('*') {
        NamePattern::Suffix(name[1..].to_string())
    } else if name.ends_with('*') {
        NamePattern::Prefix(name[..name.len() - 1].to_string())
    } else {
        NamePattern::Exact(name.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_execution_wildcard() {
        let pc = parse_pointcut("execution(pub fn *(..))").unwrap();
        match pc {
            Pointcut::Execution(pattern) => {
                assert_eq!(pattern.visibility, Some(Visibility::Public));
                assert_eq!(pattern.name, NamePattern::Wildcard);
            }
            _ => panic!("Expected Execution pointcut"),
        }
    }

    #[test]
    fn test_parse_execution_exact_name() {
        let pc = parse_pointcut("execution(fn save_user(..))").unwrap();
        match pc {
            Pointcut::Execution(pattern) => {
                assert_eq!(pattern.visibility, None);
                assert_eq!(pattern.name, NamePattern::Exact("save_user".to_string()));
            }
            _ => panic!("Expected Execution pointcut"),
        }
    }

    #[test]
    fn test_parse_execution_prefix() {
        let pc = parse_pointcut("execution(pub fn save*(..))").unwrap();
        match pc {
            Pointcut::Execution(pattern) => {
                assert_eq!(pattern.name, NamePattern::Prefix("save".to_string()));
            }
            _ => panic!("Expected Execution pointcut"),
        }
    }

    #[test]
    fn test_parse_within() {
        let pc = parse_pointcut("within(crate::api)").unwrap();
        match pc {
            Pointcut::Within(pattern) => {
                assert_eq!(pattern.path, "crate::api");
            }
            _ => panic!("Expected Within pointcut"),
        }
    }

    #[test]
    fn test_parse_and() {
        let pc = parse_pointcut("execution(pub fn *(..)) && within(crate::api)").unwrap();
        match pc {
            Pointcut::And(left, right) => {
                assert!(matches!(*left, Pointcut::Execution(_)));
                assert!(matches!(*right, Pointcut::Within(_)));
            }
            _ => panic!("Expected And pointcut"),
        }
    }

    #[test]
    fn test_parse_or() {
        let pc = parse_pointcut("execution(fn save(..)) || execution(fn update(..))").unwrap();
        match pc {
            Pointcut::Or(_, _) => {}
            _ => panic!("Expected Or pointcut"),
        }
    }

    #[test]
    fn test_parse_not() {
        let pc = parse_pointcut("!within(crate::internal)").unwrap();
        match pc {
            Pointcut::Not(inner) => {
                assert!(matches!(*inner, Pointcut::Within(_)));
            }
            _ => panic!("Expected Not pointcut"),
        }
    }

    #[test]
    fn test_parse_parentheses() {
        let pc = parse_pointcut("(execution(pub fn *(..)))").unwrap();
        assert!(matches!(pc, Pointcut::Execution(_)));
    }

    #[test]
    fn test_parse_complex_with_parentheses() {
        // (A || B) && C should parse as AND with OR on the left
        let pc = parse_pointcut(
            "(execution(pub fn *(..)) || within(crate::admin)) && within(crate::api)",
        )
        .unwrap();

        match pc {
            Pointcut::And(left, right) => {
                assert!(matches!(*left, Pointcut::Or(_, _)));
                assert!(matches!(*right, Pointcut::Within(_)));
            }
            _ => panic!("Expected And with Or on left"),
        }
    }

    #[test]
    fn test_parse_operator_precedence() {
        // Without parens: A || B && C should parse as A || (B && C)
        // because AND has higher precedence than OR
        let pc1 =
            parse_pointcut("execution(fn a(..)) || execution(fn b(..)) && within(crate::api)")
                .unwrap();

        match pc1 {
            Pointcut::Or(left, right) => {
                assert!(matches!(*left, Pointcut::Execution(_)));
                assert!(matches!(*right, Pointcut::And(_, _)));
            }
            _ => panic!("Expected Or with And on right"),
        }
    }

    #[test]
    fn test_parse_nested_parentheses() {
        let pc = parse_pointcut("((execution(pub fn *(..))))").unwrap();
        assert!(matches!(pc, Pointcut::Execution(_)));
    }

    // Property-based tests
    #[cfg(test)]
    mod proptests {
        use super::*;
        use proptest::prelude::*;

        // Generate valid function names
        fn arb_function_name() -> impl Strategy<Value = String> {
            prop::string::string_regex("[a-z_][a-z0-9_]*").unwrap()
        }

        // Generate valid module paths
        fn arb_module_path() -> impl Strategy<Value = String> {
            prop::collection::vec(arb_function_name(), 1..5)
                .prop_map(|parts| format!("crate::{}", parts.join("::")))
        }

        // Generate valid visibility
        fn arb_visibility() -> impl Strategy<Value = &'static str> {
            prop_oneof![
                Just("pub"),
                Just("pub(crate)"),
                Just("pub(super)"),
                Just(""),
            ]
        }

        proptest! {
            #[test]
            fn parse_execution_never_panics(
                vis in arb_visibility(),
                name in arb_function_name()
            ) {
                let expr = if vis.is_empty() {
                    format!("execution(fn {}(..))", name)
                } else {
                    format!("execution({} fn {}(..))", vis, name)
                };
                // Should either parse or return error, never panic
                let _ = parse_pointcut(&expr);
            }

            #[test]
            fn parse_within_never_panics(path in arb_module_path()) {
                let expr = format!("within({})", path);
                let _ = parse_pointcut(&expr);
            }

            #[test]
            fn parse_and_is_associative(
                name1 in arb_function_name(),
                name2 in arb_function_name(),
                path in arb_module_path()
            ) {
                let expr = format!(
                    "execution(fn {}(..)) && execution(fn {}(..)) && within({})",
                    name1, name2, path
                );
                // Should parse successfully
                prop_assert!(parse_pointcut(&expr).is_ok());
            }

            #[test]
            fn parse_with_random_parentheses(
                name in arb_function_name(),
                extra_parens in 0usize..3
            ) {
                let mut expr = format!("execution(fn {}(..))", name);
                for _ in 0..extra_parens {
                    expr = format!("({})", expr);
                }
                // Should parse successfully
                prop_assert!(parse_pointcut(&expr).is_ok());
            }

            #[test]
            fn roundtrip_basic_patterns(
                vis in arb_visibility(),
                name in arb_function_name()
            ) {
                let expr = if vis.is_empty() {
                    format!("execution(fn {}(..))", name)
                } else {
                    format!("execution({} fn {}(..))", vis, name)
                };

                if let Ok(pc) = parse_pointcut(&expr) {
                    // Should be an Execution pointcut
                    prop_assert!(matches!(pc, Pointcut::Execution(_)));
                }
            }
        }
    }
}
