use mx_base::util::double;

use crate::model::Square;

pub fn run(s: &Square) -> u32 {
    double(s.0)
}

#[cfg(test)]
mod tests {
    use mx_base::shape::unit;

    #[test]
    fn uses_shape() {
        assert_eq!(unit(), 1);
    }
}
