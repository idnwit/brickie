//! The brick tree as the viewer draws it: one [`LevelView`] per level (the root and every
//! composite), each with its children, the faded outside neighbours its boundary edges reach
//! and the edges aggregated for that level. The UI only picks a level; nothing is lifted there.

mod edges;
mod heat;
mod level;
#[cfg(test)]
mod tests;

use brickie_check::check::Loaded;
use brickie_check::config::Config;
use brickie_ui::LevelView;

/// Every level, the root first, then depth first in config order.
pub fn levels(l: &Loaded) -> Vec<LevelView> {
    let ctx = level::Ctx::new(l);
    let mut out = Vec::new();
    walk(&l.cfg, &ctx, &mut out);
    out
}

fn walk(cfg: &Config, ctx: &level::Ctx, out: &mut Vec<LevelView>) {
    out.push(ctx.level(cfg));
    for b in &cfg.bricks {
        if let Some(sub) = &b.sub {
            walk(sub, ctx, out);
        }
    }
}
