//! Scope analysis for accurate variable tracking
//! 
//! Tracks variable declarations and references across different scope levels
//! to distinguish between local variables and global API references.

use std::collections::{HashMap, HashSet};

/// Scope analyzer for tracking variable bindings
pub struct ScopeAnalyzer {
    scopes: Vec<Scope>,
    current_scope: usize,
}

#[derive(Debug, Clone)]
struct Scope {
    parent: Option<usize>,
    kind: ScopeKind,
    bindings: HashSet<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeKind {
    Global,
    Function,
    Block,
    Module,
}

impl ScopeAnalyzer {
    /// Create a new scope analyzer starting with a global scope
    pub fn new() -> Self {
        Self {
            scopes: vec![Scope {
                parent: None,
                kind: ScopeKind::Global,
                bindings: HashSet::new(),
            }],
            current_scope: 0,
        }
    }
    
    /// Enter a new scope
    pub fn enter_scope(&mut self, kind: ScopeKind) {
        let parent = self.current_scope;
        self.scopes.push(Scope {
            parent: Some(parent),
            kind,
            bindings: HashSet::new(),
        });
        self.current_scope = self.scopes.len() - 1;
    }
    
    /// Exit the current scope
    pub fn exit_scope(&mut self) {
        if let Some(parent) = self.scopes[self.current_scope].parent {
            self.current_scope = parent;
        }
    }
    
    /// Declare a variable in the current scope
    pub fn declare(&mut self, name: &str) {
        self.scopes[self.current_scope]
            .bindings
            .insert(name.to_string());
    }
    
    /// Check if a variable is locally bound (shadowing global)
    /// When in global scope, returns false (global vars are not "local")
    /// When in nested scope, returns true if variable is accessible in scope chain
    pub fn is_local(&self, name: &str) -> bool {
        // If we're in global scope, nothing is "local"
        if self.scopes[self.current_scope].kind == ScopeKind::Global {
            return false;
        }
        
        // Otherwise, check if variable is in scope chain
        let mut scope_id = Some(self.current_scope);
        while let Some(id) = scope_id {
            if self.scopes[id].bindings.contains(name) {
                return true;
            }
            scope_id = self.scopes[id].parent;
        }
        false
    }
    
    /// Check if a variable is global
    pub fn is_global(&self, name: &str) -> bool {
        !self.is_local(name)
    }
    
    /// Get all variables declared in the global scope
    pub fn find_globals(&self) -> Vec<String> {
        self.scopes[0].bindings.iter().cloned().collect()
    }
    
    /// Get the current scope kind
    pub fn current_scope_kind(&self) -> ScopeKind {
        self.scopes[self.current_scope].kind
    }
    
    /// Reset the scope analyzer to initial state
    pub fn reset(&mut self) {
        self.scopes.clear();
        self.scopes.push(Scope {
            parent: None,
            kind: ScopeKind::Global,
            bindings: HashSet::new(),
        });
        self.current_scope = 0;
    }
}

impl Default for ScopeAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_scope_hierarchy() {
        let mut analyzer = ScopeAnalyzer::new();
        
        // Global scope
        analyzer.declare("globalVar");
        assert!(analyzer.is_global("globalVar"));
        assert!(!analyzer.is_local("globalVar"));
        
        // Enter function scope
        analyzer.enter_scope(ScopeKind::Function);
        analyzer.declare("localVar");
        
        assert!(analyzer.is_local("localVar"));
        assert!(analyzer.is_local("globalVar")); // Also visible here
        assert!(analyzer.is_global("undeclared"));
        
        // Exit back to global
        analyzer.exit_scope();
        assert!(!analyzer.is_local("localVar")); // Not visible anymore
        assert!(analyzer.is_global("localVar"));
    }
    
    #[test]
    fn test_shadowing() {
        let mut analyzer = ScopeAnalyzer::new();
        
        analyzer.declare("chrome");
        assert!(analyzer.is_global("chrome")); // In global scope, declared vars are "global"
        
        analyzer.enter_scope(ScopeKind::Function);
        assert!(analyzer.is_local("chrome")); // Global chrome is accessible as "local" in nested scope
        
        analyzer.declare("chrome"); // Shadow it locally
        assert!(analyzer.is_local("chrome")); // Still local (shadowed)
        
        analyzer.exit_scope();
        assert!(analyzer.is_global("chrome")); // Back to global scope, so it's "global" again
    }
    
    #[test]
    fn test_nested_scopes() {
        let mut analyzer = ScopeAnalyzer::new();
        
        analyzer.declare("a");
        assert!(analyzer.is_global("a")); // In global scope
        
        analyzer.enter_scope(ScopeKind::Function);
        analyzer.declare("b");
        
        analyzer.enter_scope(ScopeKind::Block);
        analyzer.declare("c");
        
        // All should be accessible in innermost scope
        assert!(analyzer.is_local("a"));
        assert!(analyzer.is_local("b"));
        assert!(analyzer.is_local("c"));
        
        analyzer.exit_scope();
        
        // c is no longer accessible, back in function scope
        assert!(analyzer.is_local("a")); // Global var accessible
        assert!(analyzer.is_local("b")); // Function var accessible
        assert!(!analyzer.is_local("c")); // Block var not accessible
        
        analyzer.exit_scope();
        
        // Back in global scope - only a is declared here
        assert!(analyzer.is_global("a")); // a was declared in global
        assert!(!analyzer.is_local("b")); // b not accessible
        assert!(!analyzer.is_local("c")); // c not accessible
    }
    
    #[test]
    fn test_find_globals() {
        let mut analyzer = ScopeAnalyzer::new();
        
        analyzer.declare("global1");
        analyzer.declare("global2");
        
        analyzer.enter_scope(ScopeKind::Function);
        analyzer.declare("local1");
        
        let globals = analyzer.find_globals();
        assert_eq!(globals.len(), 2);
        assert!(globals.contains(&"global1".to_string()));
        assert!(globals.contains(&"global2".to_string()));
        assert!(!globals.contains(&"local1".to_string()));
    }
}