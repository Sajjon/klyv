use std::str::FromStr;

/// Doc about AaaaStructA
#[derive(Clone, Debug)]
pub struct AaaaStructA;

/// Doc about AaaaStructB
#[derive(Clone, Debug)]
pub struct AaaaStructB {
    /// Doc about `aaaa_struct_a`
    aaaa_struct_a: AaaaStructA,
}

impl AaaaStructB {
    /// Magic
    pub fn magic() -> u8 {
        42
    }
}

impl AaaaStructB {
    /// Magic self
    pub fn magic_self(self) -> u8 {
        42
    }
}

impl AaaaStructB {
    /// Magic self ref
    pub fn magic_self_ref(&self) -> u8 {
        42
    }
}

impl AaaaStructB {
    /// Magic self ref mut
    pub fn magic_self_ref_mut(&mut self) -> u8 {
        42
    }
}

impl AaaaStructB {
    /// Magic self ref arg
    pub fn magic_self_ref_arg(&self, magic: u8) -> u8 {
        magic
    }
}

pub trait MagicTrait: Clone + std::fmt::Debug {
    /// Magic
    fn trait_magic() -> u8 {
        42
    }
    /// Magic self
    fn trait_magic_self(self) -> u8;
    /// Magic self ref
    fn trait_magic_self_ref(&self) -> u8;
    /// Magic self ref mut
    fn trait_magic_self_ref_mut(&mut self) -> u8;
    /// Magic self ref arg
    fn trait_magic_self_ref_arg(&self, magic: u8) -> u8;
}

impl MagicTrait for AaaaStructB {
    /// Magic self
    fn trait_magic_self(self) -> u8 {
        42
    }

    /// Magic self ref
    fn trait_magic_self_ref(&self) -> u8 {
        42
    }

    /// Magic self ref mut
    fn trait_magic_self_ref_mut(&mut self) -> u8 {
        42
    }

    /// Magic self ref arg
    fn trait_magic_self_ref_arg(&self, magic: u8) -> u8 {
        magic
    }
}

/// Doc about AaaaEnum
#[derive(Clone, Debug)]
pub enum AaaaEnum {
    /// Doc about AaaaA variant
    AaaaA(AaaaStructA),
    /// Doc about AaaaB variant
    AaaaB(AaaaStructB),
}

impl AaaaEnum {
    /// Much magic
    /// Very good number
    /// 42 is magic
    pub fn magic() -> u8 {
        42
    }

    /// More magic
    /// Very good number
    /// 42 is magic
    pub fn more_magic() -> u8 {
        42
    }
}
