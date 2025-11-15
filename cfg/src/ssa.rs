pub mod construct;
mod destruct;
pub mod inline;
mod param_dependency_graph;
pub mod structuring;
pub mod upvalues;
pub mod analysis;      
pub mod optimization;  

pub use construct::construct;
pub use destruct::Destructor;
pub use analysis::analyze_symbols;        
pub use optimization::optimize_ast;       