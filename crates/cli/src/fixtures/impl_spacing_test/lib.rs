pub struct MyStruct {
    pub field1: i32,
    pub field2: String,
}

pub enum MyEnum {
    MyStruct(MyStruct),
    Other,
}

pub fn global_function() {
    println!("This is a global function.");
}

impl MyEnum {
    fn magic_self(&self) -> u8 {
        42
    }
    fn magic_mut_self(&mut self) -> u8 {
        42
    }

    fn magic() -> u8 {
        42
    }
}

impl MyStruct {
    pub fn new(field1: i32, field2: String) -> Self {
        MyStruct { field1, field2 }
    }

    pub fn display(&self) {
        println!(
            "MyStruct - field1: {}, field2: {}",
            self.field1, self.field2
        );
    }
}

pub trait MyTrait {
    fn my_method(&self);
}

impl MyTrait for MyStruct {
    fn my_method(&self) {
        println!(
            "MyTrait method called on MyStruct with field1: {}",
            self.field1
        );
    }
}

macro_rules! my_macro {
    () => {
        println!("Hello from my_macro!");
    };
}

pub type Error = String;

pub union MyUnion {
    pub int_value: u8,
    pub other: u8,
}

pub fn lib_sample() {
    let my_struct = MyStruct::new(10, String::from("Hello"));
    my_struct.display();

    let my_enum = MyEnum::MyStruct(my_struct);
    match my_enum {
        MyEnum::MyStruct(s) => s.my_method(),
        MyEnum::Other => println!("Other variant"),
    }

    global_function();

    let error: Error = String::from("An error occurred");

    let my_union = MyUnion { int_value: 42 };

    my_macro!();
}
