use std::fmt::Display;

pub enum DeepFoo {
    Variant1,
    Variant2,
}

impl DeepFoo {
    pub fn new() -> Self {
        DeepFoo::Variant1
    }

    pub fn do_something(&self) {
        match self {
            DeepFoo::Variant1 => println!("Doing something for Variant1"),
            DeepFoo::Variant2 => println!("Doing something for Variant2"),
        }
    }
}

impl Display for DeepFoo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeepFoo::Variant1 => write!(f, "DeepFoo::Variant1"),
            DeepFoo::Variant2 => write!(f, "DeepFoo::Variant2"),
        }
    }
}

pub struct DeepFoobar {
    pub foo: DeepFoo,
}

pub fn global_deep_frobnicate() {
    println!("Global frobnicate function called");
}

impl DeepFoobar {
    pub fn new(foo: DeepFoo) -> Self {
        DeepFoobar { foo }
    }

    pub fn frobnicate(&self) {
        println!("Foobar frobnicate called with {}", self.foo);
    }
}
