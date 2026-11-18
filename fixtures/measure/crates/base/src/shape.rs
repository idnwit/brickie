//! Shapes and their **area**.
pub trait Shape {
    fn area(&self) -> u32;
}

pub fn unit() -> u32 {
    1
}
