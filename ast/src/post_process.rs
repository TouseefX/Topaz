//! Post-processing passes for improving decompiled code quality
//!
//! This module applies various transformations to make the decompiled
//! output more readable and closer to the original source code.

use std::collections::HashSet;

use crate::{
    Block, RcLocal, compound_assign, cond_expr, context_naming, copy_fold, guard_clauses,
    table_cleanup, unused_vars,
};

/// Apply all post-processing passes to a block
pub fn apply_all(block: &mut Block) {
    apply_all_with_upvalues(block, &[]);
}

/// Like [`apply_all`], but locals in `upvalues` are incoming captures whose
/// stores must survive DCE even when this body never reads them.
pub fn apply_all_with_upvalues(block: &mut Block, upvalues: &[RcLocal]) {
    let protected: HashSet<RcLocal> = upvalues.iter().cloned().collect();
    // CameraShaker-style: `if cond then x=true else x=false` → `x = cond`,
    // DUPTABLE nils folded, loop-carried copies dropped, temps inlined.
    cond_expr::apply(block);
    crate::scope_check::note("cond_expr", block);
    table_cleanup::cleanup_table_constructors(block);
    crate::scope_check::note("table_cleanup", block);
    // A/B switch: skip copy folding in the post passes.
    if std::env::var_os("TOPAZ_AB_NO_POST_COPY_FOLD").is_none() {
        copy_fold::apply_protected(block, &protected);
    }
    crate::scope_check::note("copy_fold", block);

    // Detect and convert compound assignments (x = x + 1 -> x += 1)
    compound_assign::detect_compound_assignments(block);
    crate::scope_check::note("compound", block);
    
    // Apply guard clauses to reduce indentation and nesting
    guard_clauses::apply_guard_clauses(block);
    crate::scope_check::note("guard", block);
    
    // Context-based variable naming (extract names from GetService, string literals, etc.)
    context_naming::apply_context_naming(block);
    crate::scope_check::note("naming", block);

    // Mark unused variables with "_"
    unused_vars::mark_unused_variables(block);
    crate::scope_check::note("unused", block);
    
    // Future passes can be added here:
    // - Dead code elimination
    // - Expression simplification
    // - Control flow improvements
}

/// Apply post-processing to improve readability
pub fn improve_readability(block: &mut Block) {
    apply_all(block);
}
