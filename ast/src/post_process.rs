//! Post-processing passes for improving decompiled code quality
//!
//! This module applies various transformations to make the decompiled
//! output more readable and closer to the original source code.

use crate::{Block, compound_assign, context_naming, guard_clauses, table_cleanup, unused_vars};

/// Apply all post-processing passes to a block
pub fn apply_all(block: &mut Block) {
    // Detect and convert compound assignments (x = x + 1 -> x += 1)
    compound_assign::detect_compound_assignments(block);
    
    // Apply guard clauses to reduce indentation and nesting
    guard_clauses::apply_guard_clauses(block);
    
    // Context-based variable naming (extract names from GetService, string literals, etc.)
    context_naming::apply_context_naming(block);
    
    // Clean up table constructors with duplicate keys
    table_cleanup::cleanup_table_constructors(block);
    
    // Mark unused variables with "_"
    unused_vars::mark_unused_variables(block);
    
    // Future passes can be added here:
    // - Dead code elimination
    // - Expression simplification
    // - Control flow improvements
}

/// Apply post-processing to improve readability
pub fn improve_readability(block: &mut Block) {
    apply_all(block);
}
