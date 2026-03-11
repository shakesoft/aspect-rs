// mir_analyzer.rs - Real MIR (Mid-level IR) analysis for aspect weaving
//
// Phase 3 Week 11-12: MIR Analysis Implementation
//
// This module accesses rustc's MIR to extract function metadata automatically
// without requiring per-function annotations.

// Import rustc internal APIs
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use rustc_hir::def_id::LocalDefId;
use rustc_middle::mir::Body;
use rustc_middle::ty::TyCtxt;
use std::collections::HashMap;

use crate::types::{FunctionMetadata, SourceLocation, Visibility};

/// Analyzes MIR to extract function metadata for aspect weaving
pub struct MirAnalyzer<'tcx> {
    tcx: TyCtxt<'tcx>,
    verbose: bool,
}

impl<'tcx> MirAnalyzer<'tcx> {
    pub fn new(tcx: TyCtxt<'tcx>, verbose: bool) -> Self {
        Self { tcx, verbose }
    }

    /// Extract all function metadata from the crate
    ///
    /// This iterates through all items in the crate and extracts
    /// metadata for functions that can have aspects applied.
    pub fn extract_all_functions(&self) -> Vec<FunctionMetadata> {
        let mut functions = Vec::new();

        if self.verbose {
            println!("=== MIR Analysis ===");
            println!("Extracting function metadata from compiled code...");
        }

        // Get all local definition IDs in the crate using the map
        let hir_map = self.tcx.hir();

        // Iterate through all items
        for item_id in hir_map.items() {
            let item = hir_map.item(item_id);

            // Check if this is a function (struct variant syntax)
            if matches!(item.kind, rustc_hir::ItemKind::Fn { .. }) {
                if let Some(metadata) = self.extract_function_metadata(item_id.owner_id.def_id) {
                    if self.verbose {
                        println!("  Found function: {}", metadata.name);
                    }
                    functions.push(metadata);
                }
            }
        }

        if self.verbose {
            println!("Total functions found: {}", functions.len());
        }

        functions
    }

    /// Extract metadata for a single function
    fn extract_function_metadata(&self, def_id: LocalDefId) -> Option<FunctionMetadata> {
        let tcx = self.tcx;

        // Get the full definition path
        let def_path = tcx.def_path_str(def_id.to_def_id());

        // Get the item name
        let item_name = tcx.item_name(def_id.to_def_id()).to_string();

        // Get the module path
        let module_path = self.extract_module_path(def_id);

        // Get visibility
        let visibility = self.extract_visibility(def_id);

        // Check if async
        let is_async = self.is_async_fn(def_id);

        // Get source location
        let location = self.extract_source_location(def_id);

        // Get return type (simplified for now)
        let return_type = "()".to_string(); // TODO: Extract actual return type

        Some(FunctionMetadata {
            name: def_path,
            simple_name: item_name,
            module_path,
            visibility,
            is_async,
            generics: vec![], // TODO: Extract generic parameters
            return_type,
            location,
        })
    }

    /// Extract the module path for a definition
    fn extract_module_path(&self, def_id: LocalDefId) -> String {
        let def_path = self.tcx.def_path(def_id.to_def_id());
        let mut parts = Vec::new();

        for data in def_path.data.iter() {
            // Use Debug formatting as a fallback for extracting the name
            let name = format!("{:?}", data.data);
            // Extract just the name part (strip enum variant syntax)
            let clean_name = name.split('(').next().unwrap_or(&name).trim().to_string();
            if !clean_name.is_empty() {
                parts.push(clean_name);
            }
        }

        if parts.is_empty() {
            "crate".to_string()
        } else {
            // Remove the last part (function name) to get module path
            parts.pop();
            if parts.is_empty() {
                "crate".to_string()
            } else {
                format!("crate::{}", parts.join("::"))
            }
        }
    }

    /// Extract visibility information
    fn extract_visibility(&self, def_id: LocalDefId) -> Visibility {
        let vis = self.tcx.visibility(def_id);

        if vis.is_public() {
            Visibility::Public
        } else {
            Visibility::Private
        }
    }

    /// Check if a function is async
    fn is_async_fn(&self, def_id: LocalDefId) -> bool {
        // Check if this is an async function using the asyncness from DefKind
        use rustc_hir::def::DefKind;

        // Get the definition kind
        let def_kind = self.tcx.def_kind(def_id);

        // Check if it's a function and examine its signature
        if matches!(def_kind, DefKind::Fn | DefKind::AssocFn) {
            // Get the function signature - asyncness is encoded in the ABI
            let _fn_sig = self.tcx.fn_sig(def_id);
            // For async functions, the ABI will be different
            // For now, return false as a conservative default
            // TODO: Properly detect async functions from signature or HIR
            return false;
        }

        false
    }

    /// Extract source location
    fn extract_source_location(&self, def_id: LocalDefId) -> SourceLocation {
        let span = self.tcx.def_span(def_id);
        let source_map = self.tcx.sess.source_map();

        if let Ok(loc) = source_map.lookup_line(span.lo()) {
            let file_name = loc.sf.name.prefer_remapped_unconditionaly().to_string();
            SourceLocation {
                file: file_name,
                line: loc.line + 1, // Convert to 1-indexed
                column: 0,          // TODO: Extract actual column
            }
        } else {
            SourceLocation {
                file: "<unknown>".to_string(),
                line: 0,
                column: 0,
            }
        }
    }

    /// Check if we can access MIR for a function (for future advanced analysis)
    ///
    /// This demonstrates that MIR access is available for advanced analysis
    /// such as call-site detection, field access interception, etc.
    #[allow(dead_code)]
    fn has_mir_body(&self, def_id: LocalDefId) -> bool {
        // Check if MIR is available for this function
        // In the future, we could use this for:
        // - Call-site detection: analyzing what functions are called
        // - Field access interception: detecting field reads/writes
        // - Control flow analysis: understanding execution paths

        // For now, just verify we can access the MIR
        let _mir = self.tcx.mir_built(def_id);
        true
    }
}

/// Statistics about the analysis
#[derive(Debug, Clone, Default)]
pub struct AnalysisStats {
    pub total_functions: usize,
    pub public_functions: usize,
    pub private_functions: usize,
    pub async_functions: usize,
    pub matched_functions: usize,
}

impl AnalysisStats {
    pub fn from_functions(functions: &[FunctionMetadata]) -> Self {
        let total_functions = functions.len();
        let public_functions = functions
            .iter()
            .filter(|f| f.visibility == Visibility::Public)
            .count();
        let private_functions = total_functions - public_functions;
        let async_functions = functions.iter().filter(|f| f.is_async).count();

        Self {
            total_functions,
            public_functions,
            private_functions,
            async_functions,
            matched_functions: 0,
        }
    }

    pub fn print_summary(&self) {
        println!("\n=== Analysis Statistics ===");
        println!("Total functions: {}", self.total_functions);
        println!("  Public: {}", self.public_functions);
        println!("  Private: {}", self.private_functions);
        println!("  Async: {}", self.async_functions);
        if self.matched_functions > 0 {
            println!("  Matched by pointcuts: {}", self.matched_functions);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analysis_stats() {
        let functions = vec![FunctionMetadata {
            name: "test_fn".to_string(),
            simple_name: "test_fn".to_string(),
            module_path: "crate".to_string(),
            visibility: Visibility::Public,
            is_async: false,
            generics: vec![],
            return_type: "()".to_string(),
            location: SourceLocation {
                file: "test.rs".to_string(),
                line: 1,
                column: 0,
            },
        }];

        let stats = AnalysisStats::from_functions(&functions);
        assert_eq!(stats.total_functions, 1);
        assert_eq!(stats.public_functions, 1);
        assert_eq!(stats.private_functions, 0);
    }
}
