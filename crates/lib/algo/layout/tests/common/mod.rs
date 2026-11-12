use brickie_layout::{InBrick, InEdge, Input};

/// A graph shaped like a module-level refactor: port, core, adapters and an assembly row, with
/// long edges, same-row edges and parallel edges of different kinds.
pub fn fixture() -> Input {
    let rows = ["port", "core", "adapter", "assembly"];
    let names = [
        ("contract", 0),
        ("audio", 0),
        ("engine", 1),
        ("session", 1),
        ("mixer", 1),
        ("codec", 1),
        ("alsa", 2),
        ("pulse", 2),
        ("wasapi", 2),
        ("coreaudio", 2),
        ("null", 2),
        ("client", 3),
        ("cli", 3),
        ("ffi", 3),
        ("config", 3),
    ];
    let idx = |n: &str| names.iter().position(|x| x.0 == n).unwrap();
    let pairs = [
        ("engine", "contract", "uses"),
        ("engine", "audio", "uses"),
        ("session", "engine", "uses"),
        ("session", "contract", "uses"),
        ("mixer", "audio", "uses"),
        ("codec", "audio", "uses"),
        ("engine", "mixer", "uses"),
        ("engine", "codec", "uses"),
        ("alsa", "audio", "implements"),
        ("pulse", "audio", "implements"),
        ("wasapi", "audio", "implements"),
        ("coreaudio", "audio", "implements"),
        ("null", "audio", "implements"),
        ("alsa", "contract", "uses"),
        ("pulse", "contract", "uses"),
        ("client", "engine", "uses"),
        ("client", "session", "uses"),
        ("client", "alsa", "uses"),
        ("client", "pulse", "uses"),
        ("client", "audio", "reexports"),
        ("client", "contract", "reexports"),
        ("cli", "client", "uses"),
        ("ffi", "client", "uses"),
        ("cli", "config", "uses"),
        ("client", "config", "uses"),
        ("engine", "alsa", "uses"),
        ("mixer", "codec", "uses"),
        ("session", "mixer", "uses"),
        ("wasapi", "pulse", "uses"),
        ("null", "alsa", "uses"),
        ("ffi", "engine", "uses"),
        ("config", "contract", "uses"),
        ("engine", "audio", "implements"),
    ];
    Input {
        rows: rows.iter().map(|s| s.to_string()).collect(),
        bricks: names
            .iter()
            .map(|(n, r)| InBrick {
                id: n.to_string(),
                row: *r,
            })
            .collect(),
        edges: pairs
            .iter()
            .map(|(a, b, k)| InEdge {
                id: format!("{a}|{b}|{k}"),
                a: idx(a),
                b: idx(b),
            })
            .collect(),
    }
}
