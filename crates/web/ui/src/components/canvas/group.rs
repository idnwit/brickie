//! Peer-group rendering inside a brick container: header violation badge, member cards (with
//! drag-out-to-ungroup and double-click zoom for composite members), and intra-group violation
//! connectors.

use super::brick::{grabs, is_double_click};
use super::style::fit_name;
use crate::state::*;
use brickie_web_api::{
    BrickView, GROUP_HEADER_H, GroupMemberView, InternalEdgeView, fmt_score, group_member_rect,
    heat_len, heat_level,
};
use sycamore::prelude::*;

/// Violation count badge in a peer group's header when intra-group forbidden edges exist.
pub(super) fn internal_viol_badge(brick: &BrickView, w: f64) -> View {
    let n = brick.internal_edges.len();
    let label = format!("{n}!int");
    let bw = (10 + 6 * label.len()).max(36) as f64;
    let x = (w - bw - 42.0).max(8.0);
    let tx = x + bw / 2.0;
    let tip = format!("{n} forbidden intra-group edge(s)");
    view! {
        g(class="sbadge err ibadge") {
            svg_title { (tip) }
            rect(x=x.to_string(), y="7", width=bw.to_string(), height="18", rx="9")
            text(x=tx.to_string(), y="20", "text-anchor"="middle") { (label) }
        }
    }
}

/// Member cards (or empty drop placeholder) and internal red violation connectors inside a group.
pub(super) fn group_contents(cx: &Cx, b: usize, brick: &BrickView, w: f64, h: f64) -> View {
    if brick.group_members.is_empty() {
        let tx = (w / 2.0).to_string();
        let ty = ((GROUP_HEADER_H + h) / 2.0 + 4.0).to_string();
        return view! {
            text(class="grp-empty", x=tx, y=ty, "text-anchor"="middle") { "drop bricks here" }
        };
    }
    let cards: Vec<View> = brick
        .group_members
        .iter()
        .enumerate()
        .map(|(mi, m)| member_card(cx, b, mi, &brick.name, m))
        .collect();
    let edges: Vec<View> = brick
        .internal_edges
        .iter()
        .filter_map(|ie| internal_edge_view(&brick.group_members, ie))
        .collect();
    let cards = View::from(cards);
    let edges = View::from(edges);
    view! { (cards) (edges) }
}

/// One clickable member card inside a peer group container.
fn member_card(cx: &Cx, b: usize, mi: usize, group_name: &str, m: &GroupMemberView) -> View {
    let (mx, my, mw, mh) = group_member_rect(mi);
    let cls = {
        let cx = cx.clone();
        let has_err = m.errors > 0;
        let is_comp = m.is_composite;
        move || {
            let mut c = String::from("mcard");
            if is_comp {
                c += " comp";
            }
            if has_err {
                c += " bd-red";
            }
            if cx.eval.with(|ev| ev.sel == Sel::Member(b, mi)) {
                c += " sel";
            }
            c
        }
    };
    let name = fit_name(&m.name, mw + 12.0);
    let tip = format!(
        "{} (in {})\nscore {} ({} finding(s), worst {}\u{00D7}){}",
        m.name,
        group_name,
        fmt_score(m.score),
        m.errors,
        fmt_score(m.worst_ratio),
        if m.is_composite {
            "\ndouble-click to zoom in"
        } else {
            ""
        }
    );
    let track = mw - 12.0;
    let fill = (track * heat_len(m.worst_ratio)).max(if m.worst_ratio > 0.0 { 2.0 } else { 0.0 });
    let hcls = format!("heat h{}", heat_level(m.worst_ratio));
    let badge = if m.errors > 0 {
        let lbl = format!("{}E", m.errors);
        let bw = (8 + 6 * lbl.len()).max(20) as f64;
        let bx = mx + mw - bw - 4.0;
        let btx = bx + bw / 2.0;
        let by = my + 5.0;
        let bty = my + 16.0;
        view! {
            g(class="sbadge err") {
                rect(x=bx.to_string(), y=by.to_string(), width=bw.to_string(), height="14", rx="7")
                text(x=btx.to_string(), y=bty.to_string(), "text-anchor"="middle", "font-size"="10") { (lbl) }
            }
        }
    } else {
        View::default()
    };
    let zoom_mark = if m.is_composite {
        let c = cx.clone();
        let target = m.path.clone();
        let zx = mx + mw - 14.0;
        let zy = my + mh - 12.0;
        view! {
            g(class="zicon", transform=format!("translate({zx},{zy})"),
              on:mousedown=move |e: Click| e.stop_propagation(),
              on:click=move |e: Click| {
                  e.stop_propagation();
                  c.go(c.params.with(|p| p.at_level(&target)));
              }) {
                svg_title { "Zoom in" }
                rect(class="zhit", x="-3", y="-3", width="16", height="16")
                circle(cx="4", cy="4", r="3.5")
                line(x1="6.8", y1="6.8", x2="10", y2="10")
            }
        }
    } else {
        View::default()
    };
    let (c1, c2) = (cx.clone(), cx.clone());
    let gname = group_name.to_string();
    let mname = m.name.clone();
    let mousedown = move |e: Click| {
        if !grabs(&c1, &e) {
            return;
        }
        e.stop_propagation();
        if c1.editing() {
            c1.moved.set(false);
            let at = (e.client_x() as f64, e.client_y() as f64);
            c1.ed.carry.set(Some((
                Carry::Member {
                    group: gname.clone(),
                    name: mname.clone(),
                    sx: at.0,
                    sy: at.1,
                },
                at.0,
                at.1,
            )));
        }
    };
    let target = m.is_composite.then(|| m.path.clone());
    let click = move |e: Click| {
        e.stop_propagation();
        if c2.moved.get() {
            c2.moved.set(false);
            return;
        }
        let key = b * 10_000 + mi + 1;
        if is_double_click(key, e.time_stamp())
            && let Some(t) = &target
        {
            c2.go(c2.params.with(|p| p.at_level(t)));
            return;
        }
        let sel = c2.eval.with(|ev| ev.sel.clone());
        c2.select(sel.click_member(b, mi));
    };
    let data_member = m.name.clone();
    view! {
        g(class=cls, "data-member"=data_member, on:mousedown=mousedown, on:click=click) {
            svg_title { (tip) }
            rect(class="mbox", x=mx.to_string(), y=my.to_string(), width=mw.to_string(), height=mh.to_string(), rx="4")
            text(x=(mx + 6.0).to_string(), y=(my + 18.0).to_string(), "font-size"="12", "font-weight"="600") { (name) }
            g(class=hcls) {
                rect(class="track", x=(mx + 6.0).to_string(), y=(my + 26.0).to_string(), width=track.to_string(), height="4", rx="2")
                rect(class="hbar", x=(mx + 6.0).to_string(), y=(my + 26.0).to_string(), width=fill.to_string(), height="4", rx="2")
            }
            (badge)
            (zoom_mark)
        }
    }
}

/// Red connector line between two offending member cards inside a group.
fn internal_edge_view(members: &[GroupMemberView], ie: &InternalEdgeView) -> Option<View> {
    let fi = members.iter().position(|m| m.name == ie.from_member)?;
    let ti = members.iter().position(|m| m.name == ie.to_member)?;
    let (fx, fy, fw, fh) = group_member_rect(fi);
    let (tx, ty, tw, th) = group_member_rect(ti);
    let (x1, y1) = (fx + fw / 2.0, fy + fh / 2.0);
    let (x2, y2) = (tx + tw / 2.0, ty + th / 2.0);
    let mx = (x1 + x2) / 2.0;
    let my = (y1 + y2) / 2.0 - 12.0;
    let d = format!("M{x1:.1},{y1:.1} Q{mx:.1},{my:.1} {x2:.1},{y2:.1}");
    let tip = format!(
        "{} \u{2192} {} ({}, {} ref(s)): forbidden within group",
        ie.from_member, ie.to_member, ie.kind, ie.count
    );
    Some(view! {
        path(class="iedge", d=d, "marker-end"="url(#m-viol)") {
            svg_title { (tip) }
        }
    })
}
