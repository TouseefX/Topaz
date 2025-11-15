use std::{
    borrow::{Borrow, Cow},
    cell::RefCell,
    io::Write,
};

use ast::LocalRw;
use dot::{GraphWalk, LabelText, Labeller};

use itertools::Itertools;
use petgraph::{
    stable_graph::{EdgeIndex, NodeIndex},
    visit::{Bfs, Walker},
};

use crate::function::Function;

fn arguments(args: &Vec<(ast::RcLocal, ast::RValue)>) -> String {
    let mut s = String::new();
    for (i, (local, new_local)) in args.iter().enumerate() {
        use std::fmt::Write;
        write!(s, "{} -> {}", local, new_local).unwrap();
        if i + 1 != args.len() {
            s.push('\n');
        }
    }
    s
}

struct FunctionLabeller<'a> {
    function: &'a Function,
    counter: RefCell<usize>,
}

impl<'a> Labeller<'a, NodeIndex, EdgeIndex> for FunctionLabeller<'a> {
    fn graph_id(&'a self) -> dot::Id<'a> {
        dot::Id::new("cfg").unwrap()
    }

    fn node_label<'b>(&'b self, n: &NodeIndex) -> dot::LabelText<'b> {
        let block = self.function.block(*n).unwrap();
        let prefix = if self.function.entry() == &Some(*n) {
            "entry"
        } else {
            ""
        };
        
        let formatted_block = block
            .iter()
            .enumerate()
            .map(|(idx, s)| {
                for local in s.values() {
                    let name = &mut local.0 .0.lock().0;
                    if name.is_none() {
                        *name = Some(format!("v{}", self.counter.borrow()));
                        *self.counter.borrow_mut() += 1;
                    }
                }
                
                let mut statement_str = s.to_string();
                
                if idx > 0 {
                    match s {
                        ast::Statement::If(_) | 
                        ast::Statement::While(_) | 
                        ast::Statement::Repeat(_) |
                        ast::Statement::NumericFor(_) |
                        ast::Statement::GenericFor(_) => {
                            statement_str = format!("\n{}", statement_str);
                        }
                        _ => {}
                    }
                }
                
                statement_str
            })
            .join("\n");
        
        let label = if prefix.is_empty() {
            format!("Block {}\n{}", n.index(), formatted_block)
        } else {
            format!("Block {} ({})\n{}", n.index(), prefix, formatted_block)
        };
        
        dot::LabelText::LabelStr(label.into())
    }

    fn edge_label<'b>(&'b self, e: &EdgeIndex) -> dot::LabelText<'b> {
        let edge = self.function.graph().edge_weight(*e).unwrap();
        let args = arguments(&edge.arguments);
        
        match edge.branch_type {
            crate::block::BranchType::Unconditional => {
                if args.is_empty() {
                    dot::LabelText::LabelStr("".into())
                } else {
                    dot::LabelText::LabelStr(args.into())
                }
            }
            crate::block::BranchType::Then => {
                if args.is_empty() {
                    dot::LabelText::LabelStr("then".into())
                } else {
                    dot::LabelText::LabelStr(format!("then\n{}", args).into())
                }
            }
            crate::block::BranchType::Else => {
                if args.is_empty() {
                    dot::LabelText::LabelStr("else".into())
                } else {
                    dot::LabelText::LabelStr(format!("else\n{}", args).into())
                }
            }
        }
    }

    fn node_id(&'a self, n: &NodeIndex) -> dot::Id<'a> {
        dot::Id::new(format!("N{}", n.index())).unwrap()
    }

    fn node_shape(&'a self, _n: &NodeIndex) -> Option<LabelText<'a>> {
        Some(LabelText::LabelStr("box".into()))
    }
    
    fn node_style(&'a self, n: &NodeIndex) -> dot::Style {
        if self.function.entry() == &Some(*n) {
            dot::Style::Bold
        } else {
            dot::Style::None
        }
    }
}

impl<'a> GraphWalk<'a, NodeIndex, EdgeIndex> for FunctionLabeller<'a> {
    fn nodes(&'a self) -> dot::Nodes<'a, NodeIndex> {
        Cow::Owned(
            Bfs::new(self.function.graph(), self.function.entry().unwrap())
                .iter(self.function.graph())
                .collect::<Vec<_>>(),
        )
    }

    fn edges(&'a self) -> dot::Edges<'a, EdgeIndex> {
        Cow::Owned(self.function.graph().edge_indices().collect())
    }

    fn source(&self, e: &EdgeIndex) -> NodeIndex {
        self.function.graph().edge_endpoints(*e).unwrap().0
    }

    fn target(&self, e: &EdgeIndex) -> NodeIndex {
        self.function.graph().edge_endpoints(*e).unwrap().1
    }
}

pub fn render_to<W: Write>(function: &Function, output: &mut W) -> std::io::Result<()> {
    dot::render(
        &FunctionLabeller {
            function,
            counter: RefCell::new(1),
        },
        output,
    )
}