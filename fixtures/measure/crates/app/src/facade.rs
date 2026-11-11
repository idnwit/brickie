use mx_base::unit;
pub use mx_base::unit as again;

pub fn one() -> u32 {
    mx_base::unit() + unit()
}
