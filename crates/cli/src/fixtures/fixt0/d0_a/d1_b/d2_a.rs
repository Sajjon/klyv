use std::marker::PhantomData;

pub type Magic = u8;

/// Doc about AbAStructA
#[derive(Clone, Debug)]
pub struct AbAStructA<T> {
    /// Doc about phantom
    phantom: PhantomData<T>,
}

/// Doc about global gen magic
pub fn global_gen_magic<T>(magic: T) -> T {
    magic
}
