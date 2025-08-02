#[macro_export]
macro_rules! my_deeply_macro {
    () => {
        println!("Hello from my_macro!");
    };
}

pub trait IsDeeplyNestedBaring {
    fn bar(&self);
}

pub type DeepBar = dyn IsDeeplyNestedBaring;
