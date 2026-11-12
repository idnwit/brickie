//! Golden snapshots for `smart_route` (see `main.rs`), captured from the current code.

pub const FIXTURE: &str = "k0 Some(Chain { chain: [0, 2], up_end: B, idx: 0, cnt: 1 })
k1 Some(Chain { chain: [1, 2], up_end: B, idx: 1, cnt: 2 })
k2 Some(Bez)
k3 Some(Bez)
k4 Some(Bez)
k5 Some(Bez)
k6 Some(Arc { top: true, level: 1 })
k7 Some(Arc { top: true, level: 2 })
k8 Some(Chain { chain: [1, 15, 6], up_end: B, idx: 0, cnt: 1 })
k9 Some(Chain { chain: [1, 28, 7], up_end: B, idx: 0, cnt: 1 })
k10 Some(Bez)
k11 Some(Bez)
k12 Some(Chain { chain: [1, 27, 10], up_end: B, idx: 0, cnt: 1 })
k13 Some(Chain { chain: [0, 16, 6], up_end: B, idx: 0, cnt: 1 })
k14 Some(Chain { chain: [0, 29, 7], up_end: B, idx: 0, cnt: 1 })
k15 Some(Chain { chain: [2, 21, 11], up_end: B, idx: 0, cnt: 1 })
k16 Some(Chain { chain: [3, 22, 11], up_end: B, idx: 0, cnt: 1 })
k17 Some(Chain { chain: [6, 11], up_end: B, idx: 0, cnt: 1 })
k18 Some(Bez)
k19 Some(Chain { chain: [1, 17, 18, 11], up_end: B, idx: 0, cnt: 1 })
k20 Some(Chain { chain: [0, 19, 20, 11], up_end: B, idx: 0, cnt: 1 })
k21 Some(Arc { top: true, level: 2 })
k22 Some(Bez)
k23 Some(Arc { top: false, level: 1 })
k24 Some(Bez)
k25 Some(Bez)
k26 Some(Arc { top: false, level: 1 })
k27 Some(Arc { top: false, level: 2 })
k28 Some(Arc { top: false, level: 1 })
k29 Some(Bez)
k30 Some(Chain { chain: [2, 26, 13], up_end: B, idx: 0, cnt: 1 })
k31 Some(Chain { chain: [0, 23, 24, 14], up_end: B, idx: 0, cnt: 1 })
k32 Some(Chain { chain: [1, 2], up_end: B, idx: 0, cnt: 2 })
p0A T -52.000
p0B B 65.000
p1A T 26.000
p1B B -9.286
p2A R 0.000
p2B L 0.000
p3A T 0.000
p3B B -13.000
p4A T 13.000
p4B B 9.286
p5A T 13.000
p5B B 27.857
p6A T -26.000
p6B T -13.000
p7A T 52.000
p7B T -13.000
p8A T 0.000
p8B B -46.429
p9A T 13.000
p9B B -65.000
p10A T 0.000
p10B R 13.000
p11A T 0.000
p11B R -13.000
p12A T 0.000
p12B B 65.000
p13A T -26.000
p13B B 39.000
p14A T -13.000
p14B B 13.000
p15A T 43.333
p15B B 0.000
p16A T -21.667
p16B B -13.000
p17A T 21.667
p17B B 0.000
p18A T 0.000
p18B B -13.000
p19A T 65.000
p19B B 46.429
p20A T -43.333
p20B B -39.000
p21A T 0.000
p21B T -65.000
p22A L 0.000
p22B R 0.000
p23A B 0.000
p23B B 0.000
p24A L 0.000
p24B R 0.000
p25A B -26.000
p25B T 26.000
p26A B 13.000
p26B B 0.000
p27A B 13.000
p27B B -13.000
p28A B 0.000
p28B B 13.000
p29A L 0.000
p29B R 0.000
p30A T 0.000
p30B B 26.000
p31A T 0.000
p31B B -65.000
p32A T 0.000
p32B B -27.857
r0 | 586.555,190.000 586.555,158.000 469.333,126.000 469.333,94.000
r8 | 617.146,352.000 617.146,316.000 535.555,280.000 535.555,244.000 | 535.555,244.000 535.555,226.000 535.555,208.000 535.555,190.000 | 535.555,190.000 535.555,158.000 831.555,126.000 831.555,94.000
r19 | 628.171,514.000 628.171,478.000 838.658,442.000 845.858,406.000 | 845.858,406.000 849.458,388.000 853.058,370.000 856.658,352.000 | 856.658,352.000 863.858,316.000 1061.555,280.000 1061.555,244.000 | 1061.555,244.000 1061.555,226.000 1061.555,208.000 1061.555,190.000 | 1061.555,190.000 1061.555,158.000 924.412,126.000 924.412,94.000
r22 | 790.258,541.000 726.381,541.000 702.048,541.000 638.171,541.000
r28 | 1128.258,406.000 1128.258,442.000 470.146,442.000 470.146,406.000
r32 | 638.555,190.000 638.555,158.000 850.126,126.000 850.126,94.000
";

pub const DIAGONAL: &str = "k0 Some(Bez)
k1 Some(Bez)
k2 Some(Chain { chain: [0, 3], up_end: A, idx: 0, cnt: 1 })
k3 Some(Bez)
p0A L 0.000
p0B T 0.000
p1A B -13.000
p1B T 0.000
p2A B 13.000
p2B T 0.000
p3A R 0.000
p3B T 0.000
r0 | 280.000,67.000 199.183,67.000 115.000,85.183 115.000,166.000
r1 | 342.000,94.000 342.000,135.308 275.000,124.692 275.000,166.000
r2 | 368.000,94.000 368.000,118.000 435.000,142.000 435.000,166.000
r3 | 430.000,67.000 510.817,67.000 595.000,85.183 595.000,166.000
";

pub const CAPACITY: &str = "k0 Some(Chain { chain: [0, 3], up_end: A, idx: 0, cnt: 7 })
k1 Some(Chain { chain: [0, 3], up_end: A, idx: 1, cnt: 7 })
k2 Some(Chain { chain: [0, 3], up_end: A, idx: 2, cnt: 7 })
k3 Some(Chain { chain: [0, 3], up_end: A, idx: 3, cnt: 7 })
k4 Some(Chain { chain: [0, 3], up_end: A, idx: 4, cnt: 7 })
k5 Some(Chain { chain: [0, 3], up_end: A, idx: 5, cnt: 7 })
k6 Some(Chain { chain: [0, 3], up_end: A, idx: 6, cnt: 7 })
k7 Some(Chain { chain: [1, 3], up_end: A, idx: 4, cnt: 7 })
k8 Some(Chain { chain: [1, 3], up_end: A, idx: 5, cnt: 7 })
k9 Some(Chain { chain: [1, 3], up_end: A, idx: 6, cnt: 7 })
k10 Some(Chain { chain: [1, 3], up_end: A, idx: 0, cnt: 7 })
k11 Some(Chain { chain: [1, 3], up_end: A, idx: 1, cnt: 7 })
k12 Some(Chain { chain: [1, 3], up_end: A, idx: 2, cnt: 7 })
k13 Some(Chain { chain: [1, 3], up_end: A, idx: 3, cnt: 7 })
k14 Some(Chain { chain: [2, 3], up_end: A, idx: 0, cnt: 7 })
k15 Some(Chain { chain: [2, 3], up_end: A, idx: 1, cnt: 7 })
k16 Some(Chain { chain: [2, 3], up_end: A, idx: 2, cnt: 7 })
k17 Some(Chain { chain: [2, 3], up_end: A, idx: 3, cnt: 7 })
k18 Some(Chain { chain: [2, 3], up_end: A, idx: 4, cnt: 7 })
k19 Some(Chain { chain: [2, 3], up_end: A, idx: 5, cnt: 7 })
k20 Some(Chain { chain: [2, 3], up_end: A, idx: 6, cnt: 7 })
k21 Some(Arc { top: true, level: 1 })
k22 Some(Arc { top: false, level: 1 })
k23 Some(Arc { top: true, level: 2 })
k24 Some(Arc { top: false, level: 2 })
k25 Some(Arc { top: true, level: 3 })
p0A B -65.000
p0B T -120.000
p1A B -43.333
p1B T -109.565
p2A B -21.667
p2B T -99.130
p3A B 0.000
p3B T -88.696
p4A B 21.667
p4B T -78.261
p5A B 43.333
p5B T -67.826
p6A B 65.000
p6B T -57.391
p7A B 21.667
p7B T -5.217
p8A B 43.333
p8B T 5.217
p9A B 65.000
p9B T 15.652
p10A B -65.000
p10B T -46.957
p11A B -43.333
p11B T -36.522
p12A B -21.667
p12B T -26.087
p13A B 0.000
p13B T -15.652
p14A B -65.000
p14B T 26.087
p15A B -43.333
p15B T 36.522
p16A B -21.667
p16B T 46.957
p17A B 0.000
p17B T 57.391
p18A B 21.667
p18B T 67.826
p19A B 43.333
p19B T 78.261
p20A B 65.000
p20B T 88.696
p21A T 109.565
p21B T -26.000
p22A B -13.000
p22B B -13.000
p23A T 99.130
p23B T 0.000
p24A B 13.000
p24B B 13.000
p25A T 120.000
p25B T 26.000
r0 | 50.000,94.000 50.000,130.000 154.761,166.000 154.761,202.000
r10 | 210.000,94.000 210.000,130.000 227.805,166.000 227.805,202.000
r21 | 384.327,202.000 384.327,166.000 463.761,166.000 463.761,202.000
r22 | 261.761,256.000 261.761,292.000 476.761,292.000 476.761,256.000
r25 | 394.761,202.000 394.761,131.333 515.761,131.333 515.761,202.000
";
