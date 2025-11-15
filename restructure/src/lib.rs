#![feature(let_chains)]

use ast::{LocalRw, Reduce};
use cfg::{block::BranchType, function::Function};
use itertools::Itertools;
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::HashSet;

use petgraph::{
    algo::dominators::{simple_fast, Dominators},
    stable_graph::{EdgeIndex, NodeIndex, StableDiGraph},
    visit::*,
};
use tuple::Map;

mod conditional;
mod jump;
mod r#loop;

fn collect_gotos(block: &ast::Block, gotos: &mut FxHashSet<ast::Label>) {
    for statement in &block.0 {
        match statement {
            ast::Statement::Goto(goto) => {
                gotos.insert(goto.0.clone());
            }
            ast::Statement::If(r#if) => {
                collect_gotos(&r#if.then_block.lock(), gotos);
                collect_gotos(&r#if.else_block.lock(), gotos);
            }
            ast::Statement::While(r#while) => {
                collect_gotos(&r#while.block.lock(), gotos);
            }
            ast::Statement::Repeat(repeat) => {
                collect_gotos(&repeat.block.lock(), gotos);
            }
            ast::Statement::NumericFor(numeric_for) => {
                collect_gotos(&numeric_for.block.lock(), gotos);
            }
            ast::Statement::GenericFor(generic_for) => {
                collect_gotos(&generic_for.block.lock(), gotos);
            }
            _ => {}
        }
    }
}

// TODO: REFACTOR: move
pub fn post_dominators<N: Default, E: Default>(
    graph: &mut StableDiGraph<N, E>,
) -> Dominators<NodeIndex> {
    let exits = graph
        .node_identifiers()
        .filter(|&n| graph.neighbors(n).count() == 0)
        .collect_vec();
    let fake_exit = graph.add_node(Default::default());
    for exit in exits {
        graph.add_edge(exit, fake_exit, Default::default());
    }
    let res = simple_fast(Reversed(&*graph), fake_exit);
    assert!(graph.remove_node(fake_exit).is_some());
    res
}

struct GraphStructurer {
    pub function: Function,
    loop_headers: FxHashSet<NodeIndex>,
    label_to_node: FxHashMap<ast::Label, NodeIndex>,
}

impl GraphStructurer {
    fn collapse_temporary_assignments(block: &mut ast::Block) {
    let mut to_inline: FxHashMap<ast::RcLocal, ast::RcLocal> = FxHashMap::default();
    let mut to_remove: FxHashSet<usize> = FxHashSet::default();
    
    for i in 0..block.len().saturating_sub(1) {
        if let Some(assign1) = block[i].as_assign() {
            if assign1.prefix && assign1.left.len() == 1 && assign1.right.len() == 1 {
                if let Some(temp_local) = assign1.left[0].as_local() {
                    if let Some(assign2) = block.get(i + 1).and_then(|s| s.as_assign()) {
                        if !assign2.prefix && assign2.left.len() == 1 && assign2.right.len() == 1 {
                            if let ast::RValue::Local(right_local) = &assign2.right[0] {
                                if right_local == temp_local {
                                    let mut temp_use_count = 0;
                                    for (j, stmt) in block.iter().enumerate() {
                                        if j <= i + 1 {
                                            continue;
                                        }
                                        if stmt.values_read().contains(&temp_local) {
                                            temp_use_count += 1;
                                        }
                                    }
                                    
                                    if temp_use_count == 0 {
                                        if let Some(target_local) = assign2.left[0].as_local() {
                                            to_inline.insert(temp_local.clone(), target_local.clone());
                                            to_remove.insert(i + 1);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    for (i, statement) in block.iter_mut().enumerate() {
        if to_remove.contains(&i) {
            continue;
        }
        
        if let Some(assign) = statement.as_assign_mut() {
            if assign.prefix && assign.left.len() == 1 {
                if let Some(temp_local) = assign.left[0].as_local() {
                    if let Some(target_local) = to_inline.get(temp_local) {
                        assign.left[0] = target_local.clone().into();
                    }
                }
            }
        }
    }
    
    let mut indices: Vec<_> = to_remove.iter().copied().collect();
    indices.sort_by(|a, b| b.cmp(a));
    for idx in indices {
        block.remove(idx);
    }
    
    for statement in block.iter_mut() {
        match statement {
            ast::Statement::If(if_stmt) => {
                Self::collapse_temporary_assignments(&mut if_stmt.then_block.lock());
                Self::collapse_temporary_assignments(&mut if_stmt.else_block.lock());
            }
            ast::Statement::While(while_stmt) => {
                Self::collapse_temporary_assignments(&mut while_stmt.block.lock());
            }
            ast::Statement::Repeat(repeat) => {
                Self::collapse_temporary_assignments(&mut repeat.block.lock());
            }
            ast::Statement::NumericFor(numeric_for) => {
                Self::collapse_temporary_assignments(&mut numeric_for.block.lock());
            }
            ast::Statement::GenericFor(generic_for) => {
                Self::collapse_temporary_assignments(&mut generic_for.block.lock());
            }
            _ => {}
        }
    }
}

fn fix_misplaced_continues(block: &mut ast::Block) {
    let mut i = 0;
    while i < block.len() {
        if matches!(block.get(i), Some(ast::Statement::Break(_))) {
            if i > 0 && block.get(i - 1).is_some() {
                block.remove(i);
                continue;
            }
        }

        let mut statements_to_move: Vec<usize> = Vec::new();
        let is_while = matches!(block.get(i), Some(ast::Statement::While(_)));

        if is_while {
            let mut j = i + 1;
            while j < block.len() {
                if matches!(block.get(j), Some(ast::Statement::Break(_))) {
                    statements_to_move.push(j);
                    break;
                }

                if matches!(block.get(j), Some(
                    ast::Statement::If(_)
                    | ast::Statement::While(_)
                    | ast::Statement::Return(_)
                    | ast::Statement::Goto(_)
                )) {
                    break;
                }

                statements_to_move.push(j);
                j += 1;

                if statements_to_move.len() > 3 {
                    break;
                }
            }
        }

        let valid =
            !statements_to_move.is_empty()
            && matches!(block.get(*statements_to_move.last().unwrap()), Some(ast::Statement::Break(_)));

        if valid {
            
            let mut while_stmt = match block.remove(i) {
                ast::Statement::While(w) => w,
                other => {
                    block.insert(i, other);
                    i += 1;
                    continue;
                }
            };

            {
                let mut inner = while_stmt.block.lock();

                for &idx in statements_to_move.iter().rev() {
                    let stmt = block.remove(idx - 1);
                    inner.push(stmt);
                }
            } 

            block.insert(i, ast::Statement::While(while_stmt));
            continue;
        }

        if let Some(statement) = block.get_mut(i) {
            match statement {
                ast::Statement::If(s) => {
                    Self::fix_misplaced_continues(&mut s.then_block.lock());
                    Self::fix_misplaced_continues(&mut s.else_block.lock());
                }
                ast::Statement::While(s) => {
                    Self::fix_misplaced_continues(&mut s.block.lock());
                }
                ast::Statement::Repeat(s) => {
                    Self::fix_misplaced_continues(&mut s.block.lock());
                }
                ast::Statement::NumericFor(s) => {
                    Self::fix_misplaced_continues(&mut s.block.lock());
                }
                ast::Statement::GenericFor(s) => {
                    Self::fix_misplaced_continues(&mut s.block.lock());
                }
                _ => {}
            }
        }

        i += 1;
    }
}


fn fix_loop_boundaries(block: &mut ast::Block) {
    let mut i = 0;

    while i < block.len() {
        let mut statements_to_move: Vec<usize> = Vec::new();
        let is_while = matches!(block.get(i), Some(ast::Statement::While(_)));

        if is_while {
            let mut j = i + 1;

            while j < block.len() {
                if matches!(block.get(j), Some(ast::Statement::Break(_))) {
                    statements_to_move.push(j);
                    break;
                }

                if matches!(block.get(j), Some(
                    ast::Statement::If(_)
                    | ast::Statement::While(_)
                    | ast::Statement::Return(_)
                    | ast::Statement::Goto(_)
                )) {
                    break;
                }

                statements_to_move.push(j);
                j += 1;

                if statements_to_move.len() > 3 {
                    break;
                }
            }
        }

        let valid =
            !statements_to_move.is_empty()
            && matches!(block.get(*statements_to_move.last().unwrap()), Some(ast::Statement::Break(_)));

        if valid {
        let mut while_stmt = match block.remove(i) {
            ast::Statement::While(w) => w,
         other => {
            block.insert(i, other);
            i += 1;
        continue;
    }
};

{
        let mut inner = while_stmt.block.lock();

        for &idx in statements_to_move.iter().rev() {
        let stmt = block.remove(idx - 1);
        inner.push(stmt);
    }
}

block.insert(i, ast::Statement::While(while_stmt));

            continue;
        }

        if let Some(statement) = block.get_mut(i) {
            match statement {
                ast::Statement::If(s) => {
                    Self::fix_loop_boundaries(&mut s.then_block.lock());
                    Self::fix_loop_boundaries(&mut s.else_block.lock());
                }
                ast::Statement::While(s) => {
                    Self::fix_loop_boundaries(&mut s.block.lock());
                }
                ast::Statement::Repeat(s) => {
                    Self::fix_loop_boundaries(&mut s.block.lock());
                }
                ast::Statement::NumericFor(s) => {
                    Self::fix_loop_boundaries(&mut s.block.lock());
                }
                ast::Statement::GenericFor(s) => {
                    Self::fix_loop_boundaries(&mut s.block.lock());
                }
                _ => {}
            }
        }

        i += 1;
    }
}



      fn clean_variable_declarations(block: &mut ast::Block) {
        for statement in block.iter_mut() {
            match statement {
                ast::Statement::If(if_stmt) => {
                    Self::clean_variable_declarations(&mut if_stmt.then_block.lock());
                    Self::clean_variable_declarations(&mut if_stmt.else_block.lock());
                }
                ast::Statement::While(while_stmt) => {
                    Self::clean_variable_declarations(&mut while_stmt.block.lock());
                }
                ast::Statement::Repeat(repeat) => {
                    Self::clean_variable_declarations(&mut repeat.block.lock());
                }
                ast::Statement::NumericFor(numeric_for) => {
                    Self::clean_variable_declarations(&mut numeric_for.block.lock());
                }
                ast::Statement::GenericFor(generic_for) => {
                    Self::clean_variable_declarations(&mut generic_for.block.lock());
                }
                _ => {}
            }
        }
    }

    fn remove_redundant_declarations(block: &mut ast::Block) {
        let mut to_remove = Vec::new();
        
        for i in 0..block.len().saturating_sub(1) {
            if let Some(assign1) = block[i].as_assign() {
                if assign1.prefix && assign1.right.len() == 1 {
                    if let ast::RValue::Literal(ast::Literal::Nil) = &assign1.right[0] {
                        if let Some(assign2) = block.get(i + 1).and_then(|s| s.as_assign()) {
                            if !assign2.prefix && assign1.left.len() == 1 && assign2.left.len() == 1 {
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
                    Self::remove_redundant_declarations(&mut if_stmt.then_block.lock());
                    Self::remove_redundant_declarations(&mut if_stmt.else_block.lock());
                }
                ast::Statement::While(while_stmt) => {
                    Self::remove_redundant_declarations(&mut while_stmt.block.lock());
                }
                ast::Statement::Repeat(repeat) => {
                    Self::remove_redundant_declarations(&mut repeat.block.lock());
                }
                ast::Statement::NumericFor(numeric_for) => {
                    Self::remove_redundant_declarations(&mut numeric_for.block.lock());
                }
                ast::Statement::GenericFor(generic_for) => {
                    Self::remove_redundant_declarations(&mut generic_for.block.lock());
                }
                _ => {}
            }
        }
    }

    fn apply_formatting_rules(block: &mut ast::Block) {
        while block.first().map(|s| matches!(s, ast::Statement::Comment(_))).unwrap_or(false) {
            block.remove(0);
        }
        
        while block.len() > 0 {
            let last_idx = block.len() - 1;
            let should_remove = if let Some(last) = block.last() {
                matches!(last, ast::Statement::Comment(_)) && !matches!(last, ast::Statement::Return(_))
            } else {
                false
            };
            
            if should_remove {
                block.remove(last_idx);
            } else {
                break;
            }
        }
        
        for statement in block.iter_mut() {
            match statement {
                ast::Statement::If(if_stmt) => {
                    Self::apply_formatting_rules(&mut if_stmt.then_block.lock());
                    Self::apply_formatting_rules(&mut if_stmt.else_block.lock());
                }
                ast::Statement::While(while_stmt) => {
                    Self::apply_formatting_rules(&mut while_stmt.block.lock());
                }
                ast::Statement::Repeat(repeat) => {
                    Self::apply_formatting_rules(&mut repeat.block.lock());
                }
                ast::Statement::NumericFor(numeric_for) => {
                    Self::apply_formatting_rules(&mut numeric_for.block.lock());
                }
                ast::Statement::GenericFor(generic_for) => {
                    Self::apply_formatting_rules(&mut generic_for.block.lock());
                }
                _ => {}
            }
        }
    }

 fn hoist_local_definitions(block: &mut ast::Block) {
    let mut local_defs: FxHashMap<ast::RcLocal, usize> = FxHashMap::default();
    let mut local_uses: FxHashMap<ast::RcLocal, Vec<usize>> = FxHashMap::default();
    
    for (idx, statement) in block.iter().enumerate() {
        if let Some(assign) = statement.as_assign() {
            if assign.prefix {
                for lvalue in &assign.left {
                    if let Some(local) = lvalue.as_local() {
                        local_defs.insert(local.clone(), idx);
                    }
                }
            }
        }
        
        for local in statement.values_read() {
            local_uses.entry(local.clone()).or_default().push(idx);
        }
    }
    
    let mut to_hoist: Vec<ast::RcLocal> = Vec::new();
    
    for (local, def_idx) in &local_defs {
        if let Some(uses) = local_uses.get(local) {
            for &use_idx in uses {
                if use_idx < *def_idx {
                    to_hoist.push(local.clone());
                    break;
                }
            }
        }
    }
    
    if !to_hoist.is_empty() {
        let mut hoisted = Vec::new();
        for local in to_hoist {
            let mut assign = ast::Assign::new(
                vec![local.clone().into()],
                vec![ast::RValue::Literal(ast::Literal::Nil)],
            );
            assign.prefix = true;
            hoisted.push(assign.into());
        }
        
        let original = std::mem::take(&mut block.0);
        block.0 = hoisted;
        block.extend(original);
    }
}

fn enforce_local_definition_order(block: &mut ast::Block) {
    let mut first_use: FxHashMap<ast::RcLocal, usize> = FxHashMap::default();
    let mut def_index: FxHashMap<ast::RcLocal, usize> = FxHashMap::default();


    for (i, stmt) in block.iter().enumerate() {
        for local in stmt.values_read() {
            first_use.entry(local.clone())
                .and_modify(|e| *e = (*e).min(i))
                .or_insert(i);
        }
        if let Some(assign) = stmt.as_assign() {
            if assign.prefix {
                for l in &assign.left {
                    if let Some(loc) = l.as_local() {
                        def_index.insert(loc.clone(), i);
                    }
                }
            }
        }
    }

    let mut hoist_list: Vec<ast::RcLocal> = Vec::new();
    for (local, use_i) in first_use {
        if let Some(def_i) = def_index.get(&local) {
            if use_i < *def_i {
                hoist_list.push(local.clone());
            }
        }
    }

    if !hoist_list.is_empty() {
        let mut new_defs = vec![];

        for i in (0..block.len()).rev() {
            if let Some(assign) = block[i].as_assign() {
                if assign.prefix {
                    if let Some(local) = assign.left[0].as_local() {
                        if hoist_list.contains(&local) {
                            new_defs.push(block.remove(i));
                        }
                    }
                }
            }
        }

        new_defs.reverse();
        block.0.splice(0..0, new_defs);
    }
}


fn validate_gotos(block: &ast::Block) -> Result<(), String> {
    let mut defined_labels = FxHashSet::default();
    let mut used_labels = FxHashSet::default();
    
    fn collect_labels(block: &ast::Block, defined: &mut FxHashSet<ast::Label>, used: &mut FxHashSet<ast::Label>) {
        for statement in &block.0 {
            match statement {
                ast::Statement::Label(label) => {
                    defined.insert(label.clone());
                }
                ast::Statement::Goto(goto) => {
                    used.insert(goto.0.clone());
                }
                ast::Statement::If(r#if) => {
                    collect_labels(&r#if.then_block.lock(), defined, used);
                    collect_labels(&r#if.else_block.lock(), defined, used);
                }
                ast::Statement::While(r#while) => {
                    collect_labels(&r#while.block.lock(), defined, used);
                }
                ast::Statement::Repeat(repeat) => {
                    collect_labels(&repeat.block.lock(), defined, used);
                }
                ast::Statement::NumericFor(numeric_for) => {
                    collect_labels(&numeric_for.block.lock(), defined, used);
                }
                ast::Statement::GenericFor(generic_for) => {
                    collect_labels(&generic_for.block.lock(), defined, used);
                }
                _ => {}
            }
        }
    }
    
    collect_labels(block, &mut defined_labels, &mut used_labels);
    
    for label in &used_labels {
        if !defined_labels.contains(label) {
            return Err(format!("Goto references undefined label: {:?}", label));
        }
    }
    
    Ok(())
}

    fn find_loop_headers(&mut self) {
        self.loop_headers.clear();
        depth_first_search(
            self.function.graph(),
            Some(self.function.entry().unwrap()),
            |event| {
                if let DfsEvent::BackEdge(_, header) = event {
                    self.loop_headers.insert(header);
                }
            },
        );
    }
    fn new(function: Function) -> Self {
        let mut this = Self {
            function,
            loop_headers: FxHashSet::default(),
            label_to_node: FxHashMap::default(),
        };
        this.find_loop_headers();
        this
    }

    fn block_is_no_op(block: &ast::Block) -> bool {
        !block.iter().any(|s| s.as_comment().is_none())
    }

    fn try_convert_to_repeat_until(&mut self, entry: NodeIndex, body: NodeIndex) -> bool {
        if self.function.successor_blocks(body).count() != 2 {
            return false;
        }
        
        let (then_target, else_target) = self
            .function
            .conditional_edges(body)
            .map(|(t, e)| (t.target(), e.target()))
            .unwrap_or((NodeIndex::end(), NodeIndex::end()));
        
        let (back_is_then, exit_target) = if then_target == entry {
            (true, else_target)
        } else if else_target == entry {
            (false, then_target)
        } else {
            return false;
        };
        
        let mut entry_block = self.function.remove_block(entry).unwrap();
        let mut body_block = self.function.remove_block(body).unwrap();
        
        if let Some(if_stmt) = body_block.last_mut().and_then(|s| s.as_if_mut()) {
            let condition = if back_is_then {
                ast::Unary::new(if_stmt.condition.clone(), ast::UnaryOperation::Not)
                    .reduce_condition()
            } else {
                if_stmt.condition.clone()
            };
            
            body_block.pop();
            
            entry_block.extend(body_block.0);
            
            let repeat = ast::Repeat::new(condition, entry_block);
            
            let new_block = vec![repeat.into()].into();
            *self.function.block_mut(entry).unwrap() = new_block;
            
            self.function.set_edges(
                entry,
                vec![(exit_target, cfg::block::BlockEdge::new(cfg::block::BranchType::Unconditional))],
            );
            
            return true;
        }
        
        false
    }

    fn try_match_pattern(
        &mut self,
        node: NodeIndex,
        dominators: &Dominators<NodeIndex>,
        post_dom: &Dominators<NodeIndex>,
    ) -> bool {
        let successors = self.function.successor_blocks(node).collect_vec();

        // cfg::dot::render_to(&self.function, &mut std::io::stdout()).unwrap();
        if self.try_collapse_loop(node, dominators, post_dom) {
            self.find_loop_headers();
            // println!("matched loop");
            return true;
        }

        if self.try_remove_unnecessary_condition(node) {
            return true;
        }

        let changed = match successors.len() {
            0 => false,
            1 => {
                self.match_jump(node, Some(successors[0]))
            }
            2 => {
                let (then_target, else_target) = self
                    .function
                    .conditional_edges(node)
                    .unwrap()
                    .map(|e| e.target());
                self.match_conditional(node, then_target, else_target)
            }

            _ => unreachable!(),
        };

        //println!("after");
        //dot::render_to(&self.function, &mut std::io::stdout()).unwrap();

        changed
    }

    fn match_blocks(&mut self) -> bool {
        let dfs = Dfs::new(self.function.graph(), self.function.entry().unwrap())
            .iter(self.function.graph())
            .collect::<FxHashSet<_>>();
        let mut dfs_postorder =
            DfsPostOrder::new(self.function.graph(), self.function.entry().unwrap());
        let mut dominators = simple_fast(self.function.graph(), self.function.entry().unwrap());
        let mut post_dom = post_dominators(self.function.graph_mut());

        // cfg::dot::render_to(&self.function, &mut std::io::stdout()).unwrap();

        let mut changed = false;
        while let Some(node) = dfs_postorder.next(self.function.graph()) {
            // println!("matching {:?}", node);
            let matched = self.try_match_pattern(node, &dominators, &post_dom);
            if matched {
                dominators = simple_fast(self.function.graph(), self.function.entry().unwrap());
                post_dom = post_dominators(self.function.graph_mut());
            }
            changed |= matched;
            // if matched {
            //     cfg::dot::render_to(&self.function, &mut std::io::stdout()).unwrap();
            // }
        }

        for node in self
            .function
            .graph()
            .node_indices()
            .filter(|node| !dfs.contains(node))
            .collect_vec()
        {
            if self.function.has_block(node)
                && self.function.predecessor_blocks(node).next().is_none()
            {
                if self
                    .function
                    .block(node)
                    .unwrap()
                    .first()
                    .and_then(|s| s.as_label())
                    .is_none()
                {
                    self.function.remove_block(node);
                } else {
                    //let dominators = simple_fast(self.function.graph(), node);
                    let matched = self.try_match_pattern(node, &dominators, &post_dom);
                    changed |= matched;
                }
            }
        }

        changed
    }

fn insert_goto_for_edge(&mut self, edge: EdgeIndex) {
    let Some((source, target)) = self.function.graph().edge_endpoints(edge) else {
        return;
    };
    
    let Some(edge_data) = self.function.graph().edge_weight(edge) else {
        return;
    };
    
    if edge_data.branch_type == BranchType::Unconditional
        && self.function.predecessor_blocks(target).count() == 1
        && !self.loop_headers.contains(&target)
    {
        if self.function.successor_blocks(source).count() == 1 {
            let edges = self.function.remove_edges(target);
            let block = self.function.remove_block(target).unwrap();
            self.function.block_mut(source).unwrap().extend(block.0);
            self.function.set_edges(source, edges);
            return;
        }
    }
    
    let label = ast::Label(format!("LABEL_{}", target.index()));
    
    if let Some(target_block) = self.function.block_mut(target) {
        if target_block.first().and_then(|s| s.as_label()).is_none() {
            target_block.insert(0, label.clone().into());
        } else {
            if let Some(existing_label) = target_block.first().and_then(|s| s.as_label()) {
                return;
            }
        }
    }
    
    let goto_block = self.function.new_block();
    self.function
        .block_mut(goto_block)
        .unwrap()
        .push(ast::Goto::new(label.clone()).into());

    if let Some(edge_data) = self.function.graph_mut().remove_edge(edge) {
        self.function.graph_mut().add_edge(source, goto_block, edge_data);
        self.function.graph_mut().add_edge(
            goto_block, 
            target, 
            cfg::block::BlockEdge::new(BranchType::Unconditional)
        );
    }
}

    fn remove_last_return(block: ast::Block) -> ast::Block {
        if let Some(ast::Statement::Return(last_statement)) = block.last() {
            if last_statement.values.is_empty() {
                let take = block.len() - 1;
                return block.0.into_iter().take(take).collect_vec().into();
            }
        }
        block
    }

 fn collapse(&mut self) {
    loop {
        while self.match_blocks() {}
        if self.function.graph().node_count() == 1 {
            break;
        }
        
        let mut merged_any = false;
        let nodes_to_check: Vec<_> = self.function.graph().node_indices().collect();
        
        for node in nodes_to_check {
            if !self.function.has_block(node) {
                continue;
            }
            
            if let Some(target) = self.function.successor_blocks(node).exactly_one().ok() {
                let can_merge = self.function.predecessor_blocks(target).count() == 1
                    && !self.is_loop_header(target)
                    && !self.is_for_next(target)
                    && self.function.entry() != &Some(target)
                    && !self.is_loop_header(node)
                    && !self.function.block(target)
                        .and_then(|b| b.first())
                        .and_then(|s| s.as_label())
                        .is_some();
                
                if can_merge {
                    let edges = self.function.remove_edges(target);
                    let block = self.function.remove_block(target).unwrap();
                    self.function.block_mut(node).unwrap().extend(block.0);
                    self.function.set_edges(node, edges);
                    merged_any = true;
                }
            }
        }
        
        if merged_any {
            continue;
        }
        
        break;
    }
}

    fn inline_table_assignments(block: &mut ast::Block) {
        let mut to_inline: FxHashMap<ast::RcLocal, ast::Table> = FxHashMap::default();
        let mut statements_to_remove: FxHashSet<usize> = FxHashSet::default();
        
        for (idx, statement) in block.iter().enumerate() {
            if let Some(assign) = statement.as_assign() {
                if assign.prefix 
                    && assign.left.len() == 1 
                    && assign.right.len() == 1
                {
                    if let Some(local) = assign.left[0].as_local() {
                        if let ast::RValue::Table(table) = &assign.right[0] {
                            let mut field_assignment_count = 0;
                            let mut other_usage_count = 0;
                            
                            for (i, s) in block.iter().enumerate() {
                                if i <= idx {
                                    continue;
                                }
                                
                                if let Some(assign) = s.as_assign() {
                                    if !assign.prefix 
                                        && assign.left.len() == 1 
                                        && assign.right.len() == 1
                                    {
                                        if let Some(_index) = assign.left[0].as_index() {
                                            if let ast::RValue::Local(value_local) = &assign.right[0] {
                                                if value_local == local {
                                                    field_assignment_count += 1;
                                                    continue;
                                                }
                                            }
                                        }
                                    }
                                }
                                
                                if s.values_read().contains(&local) {
                                    other_usage_count += 1;
                                }
                            }
                            
                            if field_assignment_count == 1 && other_usage_count == 0 {
                                to_inline.insert(local.clone(), table.clone());
                                statements_to_remove.insert(idx);
                            }
                        }
                    }
                }
            }
        }
        
        let mut assignments_to_remove: FxHashSet<usize> = FxHashSet::default();
        
        for (idx, statement) in block.iter().enumerate() {
            if statements_to_remove.contains(&idx) {
                continue;
            }
            
            if let Some(assign) = statement.as_assign() {
                if !assign.prefix && assign.left.len() == 1 && assign.right.len() == 1 {
                    if let Some(index) = assign.left[0].as_index() {
                        if let ast::RValue::Local(local) = &assign.right[0] {
                            if to_inline.contains_key(local) {
                                assignments_to_remove.insert(idx);
                            }
                        }
                    }
                }
            }
        }
        
        let mut parent_inlines: FxHashMap<usize, Vec<(ast::RValue, ast::Table)>> = FxHashMap::default();
        
        for (idx, statement) in block.iter().enumerate() {
            if statements_to_remove.contains(&idx) || assignments_to_remove.contains(&idx) {
                continue;
            }
            
            if let Some(assign) = statement.as_assign() {
                if assign.prefix && assign.right.len() == 1 {
                    if let ast::RValue::Table(_) = &assign.right[0] {
                        let parent_local = assign.left.get(0).and_then(|l| l.as_local());
                        
                        if let Some(parent_local) = parent_local {
                            for (future_idx, future_stmt) in block.iter().enumerate() {
                                if future_idx <= idx {
                                    continue;
                                }
                                
                                if let Some(future_assign) = future_stmt.as_assign() {
                                    if !future_assign.prefix 
                                        && future_assign.left.len() == 1 
                                        && future_assign.right.len() == 1 
                                    {
                                        if let Some(index) = future_assign.left[0].as_index() {
                                            if let ast::RValue::Local(index_local) = &*index.left {
                                                if index_local == parent_local {
                                                    if let ast::RValue::Local(value_local) = &future_assign.right[0] {
                                                        if let Some(table_to_inline) = to_inline.get(value_local) {
                                                            parent_inlines.entry(idx).or_default().push((
                                                                index.right.as_ref().clone(),
                                                                table_to_inline.clone()
                                                            ));
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        for (idx, statement) in block.iter_mut().enumerate() {
            if let Some(inlines) = parent_inlines.get(&idx) {
                if let Some(assign) = statement.as_assign_mut() {
                    if let ast::RValue::Table(parent_table) = &mut assign.right[0] {
                        for (key, table) in inlines {
                            parent_table.0.push((
                                Some(key.clone()),
                                ast::RValue::Table(table.clone())
                            ));
                        }
                    }
                }
            }
        }
        
        statements_to_remove.extend(assignments_to_remove);
        
        let mut indices: Vec<_> = statements_to_remove.iter().copied().collect();
        indices.sort_by(|a, b| b.cmp(a));
        for idx in indices {
            block.remove(idx);
        }
        
        for statement in block.iter_mut() {
            match statement {
                ast::Statement::If(if_stmt) => {
                    Self::inline_table_assignments(&mut if_stmt.then_block.lock());
                    Self::inline_table_assignments(&mut if_stmt.else_block.lock());
                }
                ast::Statement::While(while_stmt) => {
                    Self::inline_table_assignments(&mut while_stmt.block.lock());
                }
                ast::Statement::Repeat(repeat) => {
                    Self::inline_table_assignments(&mut repeat.block.lock());
                }
                ast::Statement::NumericFor(numeric_for) => {
                    Self::inline_table_assignments(&mut numeric_for.block.lock());
                }
                ast::Statement::GenericFor(generic_for) => {
                    Self::inline_table_assignments(&mut generic_for.block.lock());
                }
                _ => {}
            }
        }
    }

    fn optimize_nested_ifs(block: &mut ast::Block) {
        for statement in block.iter_mut() {
            match statement {
                ast::Statement::If(if_stmt) => {
                    Self::optimize_nested_ifs(&mut if_stmt.then_block.lock());
                    Self::optimize_nested_ifs(&mut if_stmt.else_block.lock());
                    
                    if if_stmt.else_block.lock().is_empty() {
                        let can_flatten = {
                            let then_block = if_stmt.then_block.lock();
                            then_block.len() == 1 
                                && then_block.first().and_then(|s| s.as_if()).is_some()
                                && then_block.first().and_then(|s| s.as_if())
                                    .map(|nested| nested.else_block.lock().is_empty())
                                    .unwrap_or(false)
                        };
                        
                        if can_flatten {
                            let (nested_condition, nested_then_block) = {
                                let then_block = if_stmt.then_block.lock();
                                let nested_if = then_block.first().unwrap().as_if().unwrap();
                                (nested_if.condition.clone(), nested_if.then_block.clone())
                            };
                            
                            let combined_condition = ast::Binary::new(
                                if_stmt.condition.clone(),
                                nested_condition,
                                ast::BinaryOperation::And,
                            ).reduce_condition();
                            
                            if_stmt.condition = combined_condition;
                            if_stmt.then_block = nested_then_block;
                        }
                    }
                }
                ast::Statement::While(while_stmt) => {
                    Self::optimize_nested_ifs(&mut while_stmt.block.lock());
                }
                ast::Statement::Repeat(repeat) => {
                    Self::optimize_nested_ifs(&mut repeat.block.lock());
                }
                ast::Statement::NumericFor(numeric_for) => {
                    Self::optimize_nested_ifs(&mut numeric_for.block.lock());
                }
                ast::Statement::GenericFor(generic_for) => {
                    Self::optimize_nested_ifs(&mut generic_for.block.lock());
                }
                _ => {}
            }
        }
    }

fn get_or_create_label(&mut self, node: NodeIndex) -> ast::Label {
    if let Some(block) = self.function.block_mut(node) {
        if let Some(first_stmt) = block.first() {
            if let Some(label) = first_stmt.as_label() {
                return label.clone();
            }
        }
        
        let label = ast::Label(format!("LABEL_{}", node.index()));
        block.insert(0, label.clone().into());
        self.label_to_node.insert(label.clone(), node);
        label
    } else {
        ast::Label(format!("LABEL_{}", node.index()))
    }
}

fn structure(mut self) -> ast::Block {
    self.collapse();
    
    if self.function.graph().node_count() != 1 {
        let mut res_block = ast::Block::default();
        let entry = self.function.entry().unwrap();
        
        let mut visited = FxHashSet::default();
        let mut ordered_blocks = Vec::new();
        let mut to_visit = vec![entry];
        
        while let Some(node) = to_visit.pop() {
            if visited.contains(&node) || !self.function.has_block(node) {
                continue;
            }
            visited.insert(node);
            ordered_blocks.push(node);
            
            let successors: Vec<_> = self.function.successor_blocks(node).collect();
            for &succ in successors.iter().rev() {
                if !visited.contains(&succ) {
                    to_visit.push(succ);
                }
            }
        }
        
        for (idx, &node) in ordered_blocks.iter().enumerate() {
            if !self.function.has_block(node) {
                continue;
            }
            
            let successors: Vec<_> = self.function.successor_blocks(node).collect();
            let mut block = self.function.remove_block(node).unwrap();
            
            let next_block = ordered_blocks.get(idx + 1).copied();
            
            match successors.len() {
                0 => {
                }
                1 => {
                    let target = successors[0];
                    
                    if Some(target) != next_block {
                        let label = self.get_or_create_label(target);
                        block.push(ast::Goto::new(label).into());
                    }
                }
                2 => {
                    let (then_edge, else_edge) = self.function.conditional_edges(node).unwrap();
                    let then_target = then_edge.target();
                    let else_target = else_edge.target();
                    
                    if let Some(if_stmt) = block.last_mut().and_then(|s| s.as_if_mut()) {
                        let then_is_next = Some(then_target) == next_block;
                        let else_is_next = Some(else_target) == next_block;
                        
                        if !then_is_next {
                            let then_empty = if_stmt.then_block.lock().is_empty();
                            if then_empty {
                                let label = self.get_or_create_label(then_target);
                                if_stmt.then_block.lock().push(ast::Goto::new(label).into());
                            } else {
                                let needs_goto = {
                                    let then_block = if_stmt.then_block.lock();
                                    !matches!(
                                        then_block.last(),
                                        Some(ast::Statement::Return(_) | ast::Statement::Goto(_) | ast::Statement::Break(_))
                                    )
                                };
                                if needs_goto {
                                    let label = self.get_or_create_label(then_target);
                                    if_stmt.then_block.lock().push(ast::Goto::new(label).into());
                                }
                            }
                        }
                        
                        if !else_is_next {
                            let else_empty = if_stmt.else_block.lock().is_empty();
                            if else_empty {
                                let label = self.get_or_create_label(else_target);
                                if_stmt.else_block.lock().push(ast::Goto::new(label).into());
                            } else {
                                let needs_goto = {
                                    let else_block = if_stmt.else_block.lock();
                                    !matches!(
                                        else_block.last(),
                                        Some(ast::Statement::Return(_) | ast::Statement::Goto(_) | ast::Statement::Break(_))
                                    )
                                };
                                if needs_goto {
                                    let label = self.get_or_create_label(else_target);
                                    if_stmt.else_block.lock().push(ast::Goto::new(label).into());
                                }
                            }
                        }
                    }
                }
                _ => unreachable!("Block has more than 2 successors"),
            }
            
            res_block.extend(block.0);
        }
        
        for node in self.function.graph().node_indices().collect::<Vec<_>>() {
            if self.function.has_block(node) {
                self.function.remove_block(node);
            }
        }

        if let Err(e) = Self::validate_gotos(&res_block) {
            eprintln!("Warning: {}", e);
        }

        Self::remove_redundant_declarations(&mut res_block);
        Self::collapse_temporary_assignments(&mut res_block);
        Self::fix_loop_boundaries(&mut res_block);
        Self::hoist_local_definitions(&mut res_block);
        Self::enforce_local_definition_order(&mut res_block);
        Self::inline_set_lists(&mut res_block);
        Self::inline_table_assignments(&mut res_block);
        Self::convert_to_elseif_chains(&mut res_block);
        Self::remove_duplicate_conditions(&mut res_block);
        Self::optimize_nested_ifs(&mut res_block);
        Self::clean_variable_declarations(&mut res_block);
        Self::apply_formatting_rules(&mut res_block);
        res_block
        } else {
            let mut final_block = Self::remove_last_return(
            self.function
            .remove_block(self.function.entry().unwrap())
            .unwrap(),
    );
    Self::remove_redundant_declarations(&mut final_block);
    Self::collapse_temporary_assignments(&mut final_block);
    Self::fix_loop_boundaries(&mut final_block);
    Self::inline_set_lists(&mut final_block);
    Self::inline_table_assignments(&mut final_block);
    Self::convert_to_elseif_chains(&mut final_block);
    Self::optimize_nested_ifs(&mut final_block);
    Self::clean_variable_declarations(&mut final_block);
    Self::apply_formatting_rules(&mut final_block);
    final_block
}
}

    
    fn convert_to_elseif_chains(block: &mut ast::Block) {
        let mut i = 0;
        while i < block.len() {
            let should_chain = if let Some(if_stmt) = block[i].as_if() {
                if_stmt.else_block.lock().is_empty() && i + 1 < block.len()
            } else {
                false
            };
            
            if should_chain {
                let mut chain_count = 0;
                for j in (i + 1)..block.len() {
                    if let Some(next_if) = block[j].as_if() {
                        if next_if.else_block.lock().is_empty() {
                            chain_count += 1;
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }
                
                if chain_count > 0 {
                    for j in (1..=chain_count).rev() {
                        let next_if_stmt = block.remove(i + j).into_if().unwrap();
                        
                        let target_if = block.get_mut(i).unwrap().as_if_mut().unwrap();
                        let mut current_else = &mut target_if.else_block;
                        
                        loop {
                            let is_empty = current_else.lock().is_empty();
                            if is_empty {
                                break;
                            }
                            
                            let has_nested_if = {
                                let lock = current_else.lock();
                                lock.len() == 1 && lock.first().and_then(|s| s.as_if()).is_some()
                            };
                            
                            if !has_nested_if {
                                break;
                            }
                            
                            let next_else = {
                                let mut lock = current_else.lock();
                                let nested_if = lock.first_mut().unwrap().as_if_mut().unwrap();
                                &mut nested_if.else_block as *mut _
                            };
                            current_else = unsafe { &mut *next_else };
                        }
                        
                        *current_else = parking_lot::Mutex::new(vec![next_if_stmt.into()].into()).into();
                    }
                }
            }
            
            i += 1;
        }
        
        for statement in block.iter_mut() {
            match statement {
                ast::Statement::If(if_stmt) => {
                    Self::convert_to_elseif_chains(&mut if_stmt.then_block.lock());
                    Self::convert_to_elseif_chains(&mut if_stmt.else_block.lock());
                }
                ast::Statement::While(while_stmt) => {
                    Self::convert_to_elseif_chains(&mut while_stmt.block.lock());
                }
                ast::Statement::Repeat(repeat) => {
                    Self::convert_to_elseif_chains(&mut repeat.block.lock());
                }
                ast::Statement::NumericFor(numeric_for) => {
                    Self::convert_to_elseif_chains(&mut numeric_for.block.lock());
                }
                ast::Statement::GenericFor(generic_for) => {
                    Self::convert_to_elseif_chains(&mut generic_for.block.lock());
                }
                _ => {}
            }
        }
    }


fn remove_duplicate_conditions(block: &mut ast::Block) {
    let mut seen: HashSet<String> = HashSet::new();
    let mut i = 0;

    while i < block.len() {
        if let Some(if_stmt) = block[i].as_if_mut() {

            let key = format!("{:?}", if_stmt.condition);

            if !seen.insert(key) {
                let then_block = if_stmt.then_block.lock().clone();
                block.remove(i);
                block.splice(i..i, then_block.0);
                continue;
            }

            Self::remove_duplicate_conditions(&mut if_stmt.then_block.lock());
            Self::remove_duplicate_conditions(&mut if_stmt.else_block.lock());

        }
        i += 1;
    }
}
    
    fn inline_set_lists(block: &mut ast::Block) {
        let mut to_remove: Vec<usize> = Vec::new();
        
        for i in 0..block.len() {
            if let Some(set_list) = block[i].as_set_list() {
                for j in (0..i).rev() {
                    if let Some(assign) = block[j].as_assign() {
                        if assign.prefix 
                            && assign.left.len() == 1 
                            && assign.right.len() == 1
                        {
                            if let Some(local) = assign.left[0].as_local() {
                                if local == &set_list.object_local {
                                    if let ast::RValue::Table(_) = &assign.right[0] {
                                        to_remove.push(i);
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        for &idx in to_remove.iter().rev() {
            if let Some(set_list) = block[idx].as_set_list() {
                let object_local = set_list.object_local.clone();
                let values = set_list.values.clone();
                let tail = set_list.tail.clone();
                
                for j in (0..idx).rev() {
                    if let Some(assign) = block.get_mut(j).and_then(|s| s.as_assign_mut()) {
                        if assign.left.len() == 1 {
                            if let Some(local) = assign.left[0].as_local() {
                                if local == &object_local {
                                    if let Some(table) = assign.right[0].as_table_mut() {
                                        for value in values {
                                            table.0.push((None, value));
                                        }
                                        if let Some(tail_value) = tail {
                                            table.0.push((None, tail_value));
                                        }
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
                
                block.remove(idx);
            }
        }
        
        for statement in block.iter_mut() {
            match statement {
                ast::Statement::If(if_stmt) => {
                    Self::inline_set_lists(&mut if_stmt.then_block.lock());
                    Self::inline_set_lists(&mut if_stmt.else_block.lock());
                }
                ast::Statement::While(while_stmt) => {
                    Self::inline_set_lists(&mut while_stmt.block.lock());
                }
                ast::Statement::Repeat(repeat) => {
                    Self::inline_set_lists(&mut repeat.block.lock());
                }
                ast::Statement::NumericFor(numeric_for) => {
                    Self::inline_set_lists(&mut numeric_for.block.lock());
                }
                ast::Statement::GenericFor(generic_for) => {
                    Self::inline_set_lists(&mut generic_for.block.lock());
                }
                _ => {}
            }
        }
    }
}

pub fn lift(function: cfg::function::Function) -> ast::Block {
    GraphStructurer::new(function).structure()
}
