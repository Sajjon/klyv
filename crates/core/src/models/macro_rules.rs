use crate::prelude::*;

#[derive(Clone, Deref, From)]
pub struct MacroRules(ItemMacro);

impl std::fmt::Debug for MacroRules {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("MacroRules")
            .field(&self.ident)
            .field(&self.attrs)
            .field(&self.mac)
            .finish()
    }
}
