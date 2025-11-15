use rustc_hash::FxHashSet;
pub struct SymbolTable {
    pub globals: FxHashSet<String>,
    pub locals: FxHashSet<String>,
    pub builtins: FxHashSet<String>,
}

impl SymbolTable {
    pub fn new() -> Self {
        let mut builtins = FxHashSet::default();
        
        for global in &[
            "game", "workspace", "script", "Enum", "wait", "print", "warn", "error",
            "typeof", "type", "pairs", "ipairs", "next", "tonumber", "tostring",
            "pcall", "xpcall", "select", "unpack", "coroutine", "table", "math",
            "string", "bit32", "utf8", "os", "debug", "tick", "time",
            "Players", "Lighting", "ReplicatedStorage", "ServerStorage",
            "ServerScriptService", "StarterGui", "StarterPack", "StarterPlayer",
            "Teams", "SoundService", "Chat", "RunService", "UserInputService",
            "ContextActionService", "GuiService", "TextService", "CollectionService",
            "MarketplaceService", "TeleportService", "HttpService", "DataStoreService",
            "ChangeHistoryService", "SelectionService", "CoreGui", "LogService",
            "Instance", "Vector2", "Vector3", "CFrame", "UDim", "UDim2", 
            "Region3", "Color3", "BrickColor", "Ray", "Random",
        ] {
            builtins.insert(global.to_string());
        }
        
        Self {
            globals: FxHashSet::default(),
            locals: FxHashSet::default(),
            builtins,
        }
    }
    
    pub fn is_known(&self, name: &str) -> bool {
        self.builtins.contains(name) || 
        self.globals.contains(name) || 
        self.locals.contains(name)
    }
    
    pub fn scan_block(&mut self, block: &ast::Block) {
        for statement in block.iter() {
            match statement {
                ast::Statement::Assign(assign) => {
                    if assign.prefix {
                        for lval in &assign.left {
                            if let Some(local) = lval.as_local() {
                                self.locals.insert(format!("{:?}", local));
                            }
                        }
                    } else {
                        for lval in &assign.left {
                            self.globals.insert(format!("{:?}", lval));
                        }
                    }
                }
                ast::Statement::If(if_stmt) => {
                    self.scan_block(&if_stmt.then_block.lock());
                    self.scan_block(&if_stmt.else_block.lock());
                }
                ast::Statement::While(w) => {
                    self.scan_block(&w.block.lock());
                }
                ast::Statement::Repeat(r) => {
                    self.scan_block(&r.block.lock());
                }
                ast::Statement::NumericFor(f) => {
                    self.scan_block(&f.block.lock());
                }
                ast::Statement::GenericFor(f) => {
                    self.scan_block(&f.block.lock());
                }
                _ => {}
            }
        }
    }
}

pub fn analyze_symbols(block: &ast::Block) -> SymbolTable {
    let mut table = SymbolTable::new();
    table.scan_block(block);
    table
}