//! Lexical Scope Tracking.

use crate::symbol::DefId;
use std::collections::HashMap;

/// An ID identifying a lexical scope block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScopeId(pub usize);

/// The kind of scope (influences item visibility and shadowing rules).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScopeKind {
    /// The root file/module scope. Everything must have unique names here unless allowed by item rules.
    Module,
    /// A function parameter and body scope.
    Function,
    /// A standard `{ ... }` block scope.
    Block,
    /// A loop scope where `break` and `next` are valid.
    Loop,
    /// A match arm scope.
    MatchArm,
}

/// A single lexical scope containing local definitions.
#[derive(Debug)]
pub struct Scope {
    pub id: ScopeId,
    pub parent: Option<ScopeId>,
    pub kind: ScopeKind,
    /// Map of Identifier Name -> DefId.
    /// Shadowing works by a child scope defining the same name.
    bindings: HashMap<String, DefId>,
}

impl Scope {
    pub fn new(id: ScopeId, parent: Option<ScopeId>, kind: ScopeKind) -> Self {
        Self {
            id,
            parent,
            kind,
            bindings: HashMap::new(),
        }
    }

    /// Bind a name to a DefId in THIS scope.
    /// Returns the old DefId if shadowed in the *exact same scope* purely for diagnostic info.
    /// Razen allows shadowing in the same scope (`x := 1; x := 2`).
    pub fn define(&mut self, name: String, id: DefId) -> Option<DefId> {
        self.bindings.insert(name, id)
    }

    /// Check if a name is defined in this exact scope (used to detect same-scope shadowing if needed).
    pub fn get_local(&self, name: &str) -> Option<DefId> {
        self.bindings.get(name).copied()
    }
}

/// The Environment tracking all created scopes and the current active scope stack.
#[derive(Debug)]
pub struct Environment {
    scopes: Vec<Scope>,
    /// The stack of active scopes. The last element is the current inner scope.
    active: Vec<ScopeId>,
}

impl Environment {
    pub fn new() -> Self {
        Self {
            scopes: Vec::new(),
            active: Vec::new(),
        }
    }

    /// Enter a new lexical scope. Returns its ID.
    pub fn enter_scope(&mut self, kind: ScopeKind) -> ScopeId {
        let parent = self.active.last().copied();
        let id = ScopeId(self.scopes.len());
        self.scopes.push(Scope::new(id, parent, kind));
        self.active.push(id);
        id
    }

    /// Exit the current lexical scope. Panics if underflowing.
    pub fn exit_scope(&mut self) {
        assert!(self.active.pop().is_some(), "Scope underflow");
    }

    /// Get the current active scope ID.
    pub fn current_scope(&self) -> ScopeId {
        *self.active.last().expect("No active scope")
    }

    /// Add a binding to the current active scope.
    pub fn define(&mut self, name: String, id: DefId) {
        let current = self.current_scope();
        self.scopes[current.0].define(name, id);
    }

    /// Resolve an identifier name by traversing from the innermost scope outwards.
    pub fn resolve(&self, name: &str) -> Option<DefId> {
        // Walk backwards through active scopes (innermost to outermost)
        for &id in self.active.iter().rev() {
            if let Some(def_id) = self.scopes[id.0].get_local(name) {
                return Some(def_id);
            }
        }
        None
    }
}
