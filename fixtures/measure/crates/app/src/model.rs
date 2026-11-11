pub struct Square(pub u32);

// implements: the trait lives in another module.
impl mx_base::shape::Shape for Square {
    fn area(&self) -> u32 {
        self.0 * self.0
    }
}
