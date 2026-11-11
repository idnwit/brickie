pub fn straight(a: u32, b: u32) -> u32 {
    a + b
}

pub fn branches(x: i32) -> i32 {
    if x > 0 {
        1
    } else if x < 0 {
        -1
    } else {
        0
    }
}

pub fn nested(a: bool, b: bool, c: bool) {
    for i in 0..3 {
        if a && b || c {
            let _ = i;
        }
    }
}

pub fn fact(n: u64) -> u64 {
    if n < 2 { 1 } else { n * fact(n - 1) }
}

pub fn matcher(x: u32) -> u32 {
    match x {
        0 => 1,
        1 => {
            if x > 5 {
                2
            } else {
                3
            }
        }
        _ => 4,
    }
}

pub fn lookup(x: u32) -> &'static str {
    match x {
        0 => "zero",
        1 => "one",
        _ => "many",
    }
}

#[cfg(not(test))]
pub fn prod_only() {}

#[cfg(feature = "test-util")]
pub fn test_util() {}

#[cfg(any(test, feature = "x"))]
pub fn maybe_test() {}

pub fn parse(s: &str) -> Option<u32> {
    let Some(n) = s.strip_prefix('#') else {
        return None;
    };
    n.parse().ok()
}

pub struct Counter;

impl Counter {
    pub fn add(&self, n: u32) -> u32 {
        n
    }
}
