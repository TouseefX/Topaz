pub fn optimize_ast(block: &mut ast::Block) {
    remove_redundant_nil_declarations(block);
    simplify_empty_blocks(block);
}

fn remove_redundant_nil_declarations(block: &mut ast::Block) {
    let mut to_remove = Vec::new();
    
    for i in 0..block.len().saturating_sub(1) {
        if let Some(assign1) = block[i].as_assign() {
            if assign1.prefix && assign1.right.len() == 1 {
                if let ast::RValue::Literal(ast::Literal::Nil) = &assign1.right[0] {
                    if let Some(assign2) = block.get(i + 1).and_then(|s| s.as_assign()) {
                        if !assign2.prefix && 
                           assign1.left.len() == 1 && 
                           assign2.left.len() == 1 
                        {
                            if let (Some(local1), Some(local2)) = (
                                assign1.left[0].as_local(),
                                assign2.left[0].as_local()
                            ) {
                                if local1 == local2 {
                                    to_remove.push(i);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    for &idx in to_remove.iter().rev() {
        block.remove(idx);
    }
    
    for statement in block.iter_mut() {
        match statement {
            ast::Statement::If(if_stmt) => {
                remove_redundant_nil_declarations(&mut if_stmt.then_block.lock());
                remove_redundant_nil_declarations(&mut if_stmt.else_block.lock());
            }
            ast::Statement::While(w) => {
                remove_redundant_nil_declarations(&mut w.block.lock());
            }
            ast::Statement::Repeat(r) => {
                remove_redundant_nil_declarations(&mut r.block.lock());
            }
            ast::Statement::NumericFor(f) => {
                remove_redundant_nil_declarations(&mut f.block.lock());
            }
            ast::Statement::GenericFor(f) => {
                remove_redundant_nil_declarations(&mut f.block.lock());
            }
            _ => {}
        }
    }
}

fn simplify_empty_blocks(block: &mut ast::Block) {
    for statement in block.iter_mut() {
        match statement {
            ast::Statement::If(if_stmt) => {
                simplify_empty_blocks(&mut if_stmt.then_block.lock());
                simplify_empty_blocks(&mut if_stmt.else_block.lock());
            }
            ast::Statement::While(w) => {
                simplify_empty_blocks(&mut w.block.lock());
            }
            ast::Statement::Repeat(r) => {
                simplify_empty_blocks(&mut r.block.lock());
            }
            ast::Statement::NumericFor(f) => {
                simplify_empty_blocks(&mut f.block.lock());
            }
            ast::Statement::GenericFor(f) => {
                simplify_empty_blocks(&mut f.block.lock());
            }
            _ => {}
        }
    }
}