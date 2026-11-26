# Autolayout

`brickie-layout` places bricks and routes edges using a single unified heuristic optimization model
shared by both layout modes:

1. **Joint mode (`layout`)** — moves bricks horizontally and reorders them within their pinned tag
   rows while routing all edges against the same energy function $E(X, R)$.
2. **Lines-only mode (`reroute` / `rerouted_at`)** — freezes brick positions $X$ (during user drag
   or when applying stored hints) and runs the exact same route solver `optimize_routes(X)` against
   $E(X, R)$.

## Hard Invariants

Every output must satisfy these invariants with zero violations ($H_{\text{viol}} = 0$):

1. **Pinned top-down tag rows**: bricks stay in their assigned tag row (ordered top-down by
   dependency flow in `brickie-ui`) and never overlap horizontally or vertically (`GAP >= 24px`).
2. **Zero obstacle hits**: no edge segment or concentric bundle rail intersects any non-endpoint
   brick box (expanded by `CLEAR = 8px` on left/right/top and `BOT_CLEAR = 16px` on bottom for
   labels).
3. **Disjoint orthogonal tracks and ports**: every route is orthogonal with rounded corners; parallel
   tracks and ports on the same brick side maintain separation (`TRACK_GAP`, `BUNDLE_PITCH`) without
   overlapping. Under extreme user drag where bricks leave less physical room than nominal track
   gaps, tracks compress strictly inside the certified obstacle-free slab rather than hitting boxes.
4. **Deterministic and WASM-safe**: pure Rust, no `HashMap` iteration order dependencies, quantized
   integer costs in search so native SSR and `wasm32-unknown-unknown` produce bit-identical results.

## Unified Representation (Zero Bend-Count Special Cases)

Instead of separate direct/corridor routers and post-hoc port-patching passes (`apply_ports` /
`snap_term`), routing uses one unified two-level representation:

1. **Hanan Strip Graph $H(X)$ (Topology)**:
   - Grid lines are the sorted unique coordinates of all expanded brick walls plus the active
     bounding frame of connected bricks.
   - Each cell $(i, j)$ stores bitmasks of padded obstacles and raw bricks for $O(1)$ clearance
     checks.
   - 0-bend, 1-bend, 2-bend, and $N$-bend paths are all walks in $H(X)$, differing only by the
     number of $90^\circ$ turns.
2. **Segments-as-Variables (Geometry)**:
   - Collapsing collinear steps of a Hanan path yields $k + 1$ alternating H/V segments
     $s_0, \dots, s_k$, each with one scalar coordinate $u_s \in [L_s, R_s]$ inside its certified
     obstacle-free strip slab (inset by bundle half-width $hs$).
   - **Ports are the first and last segment variables ($u_0$ and $u_k$)**:
     - A 0-bend route has 1 variable $u_0$, making port Z-jogs algebraically impossible.
     - A 1-bend route has 2 variables $(u_0, u_1)$, keeping the corner orthogonal for all valid
       coordinates inside $[L, R]$.
     - Interior tracks and box-side ports are compacted together in the same `solve_axis` pass.

## Single Objective Function $E(X, R)$

$$E(X, R) = W_{\text{hard}} H_{\text{viol}} + W_{\text{cross}} C(R) + W_{\text{bend}} B(R) + W_{\text{pol}} P(R) + W_{\text{len}} L(R) + W_{\text{aln}} A(X, R)$$

| Term | Meaning | Role |
| --- | --- | --- |
| $H_{\text{viol}}$ | Box overlaps, obstacle hits, track/port overlaps | Hard invariant ($W_{\text{hard}} = 10^6$) |
| $C(R)$ | Exact orthogonal crossings (excluding shared-endpoint fans) | Minimise visual clutter ($W_{\text{cross}} \approx 450$) |
| $B(R)$ | Total $90^\circ$ bends across bundles | Prefer 0-bend > 1-bend > 2-bend ($W_{\text{bend}} \approx 320$) |
| $P(R)$ | Monotone flow polarity penalties (e.g. downward edge exiting top) | Keep top-down visual flow without blocking drag detours |
| $L(R)$ | Weighted Manhattan wirelength | Keep connections short and local |
| $A(X, R)$ | Deviation of ports/tracks from band centers | Keep bundles centered and visually balanced |

## Optimization Algorithms

### Mode 1: Lines-Only (`optimize_routes(X)`)

Runs on fixed `pos` (target `< 5–10 ms` in WASM for 60 bricks / 100 edges):

1. **Negotiated Congestion A\* (PathFinder)** on $H(X)$ (up to 5 rip-up-and-reroute passes):
   - Routes bundles shortest-first with quantized `i64` costs combining step length, bend penalty,
     polarity penalty, perpendicular crossing density (`perp_occ`), and negotiated parallel
     congestion (`occ` + monotonic history `hist`).
   - Fast $O(1)$ certificate checks whether an unobstructed, congestion-free 0-bend line exists
     before touching the priority queue.
2. **Joint Segment & Port Compaction (`solve_axis`)**:
   - Groups conflicting parallel segments (both interior tracks and port legs) on each axis.
   - Minimises exact leg crossings with `2-opt` swaps, then projects coordinates strictly inside
     $[L_s, R_s]$ using bounded isotonic regression (`pava`).
3. **Concentric Bundle Expansion**:
   - Parallel edges sharing $(a \to b)$ expand symmetrically by `BUNDLE_PITCH = 8px` inside the
     already-reserved bundle envelope $2 hs$, then render via `Route::orthogonal`.

### Mode 2: Joint Mode (`optimize_joint(X, R)`)

Runs on level load / reset (target `< 50 ms` in WASM):

1. **Seed Placement (`placement.rs`)**: crossing-minimised row order (including same-row edges) +
   `pava` horizontal placement.
2. **Conflict-Guided Local Search on Row Order (`joint.rs`)**: tests targeted swaps/shifts of
   bricks incident to crossings or $\ge 2$-bend detours, evaluating actual `optimize_routes(X, fast)`
   and accepting only strict improvements in $E(X, R)$.
3. **Port-Aligned Co-Relaxation**: shifts brick horizontal coordinates toward their actual
   connected vertical port coordinates (via `pava` with `GAP` clearance) to collapse Z/L routes
   into 0-bend straight lines, accepting only strict improvements in $E(X, R)$.
4. **Final Polish**: runs full `optimize_routes(X^*, false)` on the winning brick positions $X^*$.

## Why This Replaced the Previous Multi-Stage Pipeline

The earlier architecture (`place -> direct/corridor -> select -> pack_channels -> assign_ports ->
apply_ports -> snap_term`) was an open-loop waterfall: placement ignored wire geometry and same-row
edges, direct candidates competed separately from A\* corridors, and post-hoc port packing broke
collinearity with routed legs, forcing special-case patches (`bends == 0/1`, `snap_term`). Two
independent design reviews confirmed that unifying topology in $H(X)$ and treating ports as segment
variables eliminates those structural fractures while meeting interactive WASM latency budgets.
