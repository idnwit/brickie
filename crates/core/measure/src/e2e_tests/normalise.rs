//! Characterization of `normalise`: a hand-made two-package index, rendered row by row.

use crate::scip::layout::{Layout, Package, Target};
use crate::scip::normalise;
use crate::scip::testutil::{DEF, document, index, info, occ, render};
use ::scip::types::Index;

const A: &str = "rust-analyzer cargo alpha 0.1.0 ";
const B: &str = "rust-analyzer cargo beta 0.1.0 ";
const LIB: &str =
    "pub mod util;\npub use util::double;\npub struct Shape;\nfn local() { let x = 1; }\n";
const UTIL: &str = "pub fn double(x: u32) -> u32 { x * 2 }\npub mod inner {\n    pub fn deep() { double(1); }\n}\npub struct Pair(u32);\n";
const MAIN: &str = "use alpha::util::double;\nfn main() { double(1); }\n";

pub(super) fn layout() -> Layout {
    let target = |package, name: &str, src: &str| Target {
        package,
        name: name.into(),
        src_path: src.into(),
    };
    Layout {
        packages: vec![
            Package {
                dir: "crates/a".into(),
            },
            Package {
                dir: "crates/b".into(),
            },
        ],
        targets: vec![
            target(0, "alpha", "crates/a/src/lib.rs"),
            target(1, "beta", "crates/b/src/main.rs"),
        ],
    }
}

pub(super) fn fixture() -> Index {
    let (a, b) = (|s: &str| format!("{A}{s}"), |s: &str| format!("{B}{s}"));
    let lib = document(
        "crates/a/src/lib.rs",
        vec![
            occ(&a("crate/"), &[0, 0, 0], DEF, &[0, 0, 4, 0]),
            occ(&a("util/"), &[0, 8, 12], 0, &[]),
            occ(&a("util/"), &[1, 8, 12], 0, &[]),
            occ(&a("util/double()."), &[1, 14, 20], 0, &[]),
            occ(&a("Shape#"), &[2, 11, 16], DEF, &[2, 0, 2, 17]),
            occ(&a("local()."), &[3, 3, 8], DEF, &[3, 0, 3, 27]),
            occ("local 0", &[3, 17, 18], DEF, &[3, 17, 3, 18]),
            occ("local 0", &[3, 17, 18], 0, &[]),
            occ(
                "rust-analyzer cargo std 1.0.0 vec/Vec#",
                &[3, 20, 23],
                0,
                &[],
            ),
        ],
        vec![info(&a("Shape#"), "Shape", "A shape.", "pub struct Shape")],
    );
    let util = document(
        "crates/a/src/util.rs",
        vec![
            occ(&a("util/"), &[0, 0, 0], DEF, &[0, 0, 5, 0]),
            occ(&a("util/double()."), &[0, 7, 13], DEF, &[0, 0, 0, 38]),
            occ("local 1", &[0, 14, 15], DEF, &[0, 14, 0, 20]),
            occ(&a("util/inner/"), &[1, 8, 13], DEF, &[1, 0, 3, 1]),
            occ(&a("util/inner/deep()."), &[2, 11, 15], DEF, &[2, 4, 2, 32]),
            occ(&a("util/double()."), &[2, 20, 26], 0, &[]),
            occ(&a("util/Pair#"), &[4, 11, 15], DEF, &[4, 0, 4, 21]),
            occ(&a("util/Pair#0."), &[4, 16, 19], DEF, &[4, 0, 4, 21]),
        ],
        vec![
            info(
                &a("util/double()."),
                "double",
                "Doubles.",
                "pub fn double(x: u32) -> u32",
            ),
            info(&a("util/inner/"), "inner", "", "pub mod inner"),
            info("local 1", "x", "", ""),
        ],
    );
    let main = document(
        "crates/b/src/main.rs",
        vec![
            occ(&b("crate/"), &[0, 0, 0], DEF, &[0, 0, 2, 0]),
            occ(&a("crate/"), &[0, 4, 9], 0, &[]),
            occ(&a("util/"), &[0, 11, 15], 0, &[]),
            occ(&a("util/double()."), &[0, 17, 23], 0, &[]),
            occ(&b("main()."), &[1, 3, 7], DEF, &[1, 0, 1, 26]),
            occ(&a("util/double()."), &[1, 12, 18], 0, &[]),
        ],
        vec![],
    );
    let missing = document(
        "crates/b/src/missing.rs",
        vec![
            occ(&b("missing/"), &[0, 0, 0], DEF, &[0, 0, 2, 0]),
            occ(&b("missing/gone()."), &[1, 3, 7], DEF, &[1, 0, 1, 10]),
        ],
        vec![],
    );
    index(vec![lib, util, main, missing])
}

fn source(path: &str) -> Option<String> {
    match path {
        "crates/a/src/lib.rs" => Some(LIB.into()),
        "crates/a/src/util.rs" => Some(UTIL.into()),
        "crates/b/src/main.rs" => Some(MAIN.into()),
        _ => None, // crates/b/src/missing.rs is unreadable
    }
}

#[test]
fn graph_rows() {
    let (g, _) = normalise(&fixture(), &layout(), &source);
    assert_eq!(render(&g), EXPECTED.trim_start());
}

#[test]
fn unreadable_paths_are_reported() {
    let (_, unreadable) = normalise(&fixture(), &layout(), &source);
    assert_eq!(unreadable, ["crates/b/src/missing.rs"]);
}

const EXPECTED: &str = r#"
file 0 crates/a/src/lib.rs pkg=Some(0) target=Some(0) sha=8ff2b3c7 lines=4 module=Some(0)
file 1 crates/a/src/util.rs pkg=Some(0) target=Some(0) sha=2f0fc47c lines=5 module=Some(3)
file 2 crates/b/src/main.rs pkg=Some(1) target=Some(1) sha=1f8c8b47 lines=2 module=Some(9)
file 3 crates/b/src/missing.rs pkg=Some(1) target=Some(1) sha=e3b0c442 lines=0 module=Some(11)
sym 0 "rust-analyzer cargo alpha 0.1.0 crate/" name="alpha" file=0 range=(0, 0, 0, 0) body=(0, 0, 4, 0) mp=Some("") parent=None module=None sig="" doc=""
sym 1 "rust-analyzer cargo alpha 0.1.0 Shape#" name="Shape" file=0 range=(2, 11, 2, 16) body=(2, 0, 2, 17) mp=None parent=Some(0) module=Some(0) sig="pub struct Shape" doc="A shape."
sym 2 "rust-analyzer cargo alpha 0.1.0 local()." name="" file=0 range=(3, 3, 3, 8) body=(3, 0, 3, 27) mp=None parent=Some(0) module=Some(0) sig="" doc=""
sym 3 "rust-analyzer cargo alpha 0.1.0 util/" name="" file=1 range=(0, 0, 0, 0) body=(0, 0, 5, 0) mp=Some("util") parent=None module=Some(0) sig="" doc=""
sym 4 "rust-analyzer cargo alpha 0.1.0 util/double()." name="double" file=1 range=(0, 7, 0, 13) body=(0, 0, 0, 38) mp=None parent=Some(3) module=Some(3) sig="pub fn double(x: u32) -> u32" doc="Doubles."
sym 5 "rust-analyzer cargo alpha 0.1.0 util/inner/" name="inner" file=1 range=(1, 8, 1, 13) body=(1, 0, 3, 1) mp=Some("util/inner") parent=Some(3) module=Some(3) sig="pub mod inner" doc=""
sym 6 "rust-analyzer cargo alpha 0.1.0 util/inner/deep()." name="" file=1 range=(2, 11, 2, 15) body=(2, 4, 2, 32) mp=None parent=Some(5) module=Some(5) sig="" doc=""
sym 7 "rust-analyzer cargo alpha 0.1.0 util/Pair#" name="" file=1 range=(4, 11, 4, 15) body=(4, 0, 4, 21) mp=None parent=Some(3) module=Some(3) sig="" doc=""
sym 8 "rust-analyzer cargo alpha 0.1.0 util/Pair#0." name="" file=1 range=(4, 16, 4, 19) body=(4, 0, 4, 21) mp=None parent=Some(7) module=Some(3) sig="" doc=""
sym 9 "rust-analyzer cargo beta 0.1.0 crate/" name="beta" file=2 range=(0, 0, 0, 0) body=(0, 0, 2, 0) mp=Some("") parent=None module=None sig="" doc=""
sym 10 "rust-analyzer cargo beta 0.1.0 main()." name="" file=2 range=(1, 3, 1, 7) body=(1, 0, 1, 26) mp=None parent=Some(9) module=Some(9) sig="" doc=""
sym 11 "rust-analyzer cargo beta 0.1.0 missing/" name="" file=3 range=(0, 0, 0, 0) body=(0, 0, 2, 0) mp=Some("missing") parent=None module=Some(9) sig="" doc=""
sym 12 "rust-analyzer cargo beta 0.1.0 missing/gone()." name="" file=3 range=(1, 3, 1, 7) body=(1, 0, 1, 10) mp=None parent=Some(11) module=Some(11) sig="" doc=""
modules [0, 3, 5, 9, 11]
use 0 (1, 0)..(1, 21) pub=true vis="pub"
use 1 (0, 0)..(0, 24) pub=false vis=""
occ 0 sym=0 file=0 range=(0, 0, 0, 0) roles=1 container=None import=false use=None path_end=true written="pub"
occ 1 sym=3 file=0 range=(0, 8, 0, 12) roles=0 container=Some(0) import=false use=None path_end=true written="util"
occ 2 sym=3 file=0 range=(1, 8, 1, 12) roles=0 container=Some(0) import=true use=Some(0) path_end=false written="util"
occ 3 sym=4 file=0 range=(1, 14, 1, 20) roles=0 container=Some(0) import=true use=Some(0) path_end=true written="util::double"
occ 4 sym=1 file=0 range=(2, 11, 2, 16) roles=1 container=Some(0) import=false use=None path_end=true written="Shape"
occ 5 sym=2 file=0 range=(3, 3, 3, 8) roles=1 container=Some(0) import=false use=None path_end=true written="local"
occ 6 sym=3 file=1 range=(0, 0, 0, 0) roles=1 container=Some(4) import=false use=None path_end=true written="pub"
occ 7 sym=4 file=1 range=(0, 7, 0, 13) roles=1 container=Some(3) import=false use=None path_end=true written="double"
occ 8 sym=5 file=1 range=(1, 8, 1, 13) roles=1 container=Some(3) import=false use=None path_end=true written="inner"
occ 9 sym=6 file=1 range=(2, 11, 2, 15) roles=1 container=Some(5) import=false use=None path_end=true written="deep"
occ 10 sym=4 file=1 range=(2, 20, 2, 26) roles=0 container=Some(6) import=false use=None path_end=true written="double"
occ 11 sym=7 file=1 range=(4, 11, 4, 15) roles=1 container=Some(8) import=false use=None path_end=true written="Pair"
occ 12 sym=8 file=1 range=(4, 16, 4, 19) roles=1 container=Some(7) import=false use=None path_end=true written="u32"
occ 13 sym=9 file=2 range=(0, 0, 0, 0) roles=1 container=None import=false use=None path_end=true written="use"
occ 14 sym=0 file=2 range=(0, 4, 0, 9) roles=0 container=Some(9) import=true use=Some(1) path_end=false written="alpha"
occ 15 sym=3 file=2 range=(0, 11, 0, 15) roles=0 container=Some(9) import=true use=Some(1) path_end=false written="alpha::util"
occ 16 sym=4 file=2 range=(0, 17, 0, 23) roles=0 container=Some(9) import=true use=Some(1) path_end=true written="alpha::util::double"
occ 17 sym=10 file=2 range=(1, 3, 1, 7) roles=1 container=Some(9) import=false use=None path_end=true written="main"
occ 18 sym=4 file=2 range=(1, 12, 1, 18) roles=0 container=Some(10) import=false use=None path_end=true written="double"
occ 19 sym=11 file=3 range=(0, 0, 0, 0) roles=1 container=None import=false use=None path_end=true written=""
occ 20 sym=12 file=3 range=(1, 3, 1, 7) roles=1 container=Some(11) import=false use=None path_end=true written=""
"#;
