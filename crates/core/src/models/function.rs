use crate::prelude::*;

#[derive(Clone, Deref, From)]
pub struct Function(ItemFunction);

impl std::fmt::Debug for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Function")
            .field(&self.vis)
            .field(&self.sig)
            .field(&self.attrs)
            .finish()
    }
}
