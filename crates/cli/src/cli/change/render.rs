//! Text output of `brickie change`.

use brickie_change::{DiffEntry, Entry, OpRecord, Plan, Since, Status, Task};
use std::fmt::Write;

fn state(o: &OpRecord) -> String {
    let s = format!("{:?}", o.state).to_lowercase();
    match &o.conflict {
        Some(_) => format!("{s}, conflict"),
        None => s,
    }
}

pub fn record(o: &OpRecord) -> String {
    let mut s = format!(
        "#{} {:?} {} {} {} [{}]",
        o.seq,
        o.by,
        o.op.name(),
        o.file,
        o.key,
        state(o)
    )
    .to_lowercase();
    if let Some(n) = &o.note {
        let _ = write!(s, " note: {n}");
    }
    if let Some(r) = &o.review {
        let note = r.note.as_deref().unwrap_or("");
        let _ = write!(s, " (reviewed #{}: {note})", r.seq);
    }
    if let Some(c) = &o.conflict {
        let _ = write!(s, "\n    conflict: {c}");
    }
    s + "\n"
}

fn age(secs: Option<u64>) -> String {
    match secs {
        None => "no cache: run `brickie measure`".into(),
        Some(s) if s < 120 => format!("cache measured {s}s ago"),
        Some(s) => format!("cache measured {}m ago", s / 60),
    }
}

fn fragment(out: &mut String, sign: char, text: &Option<String>) {
    for line in text.as_deref().unwrap_or("").lines() {
        let _ = writeln!(out, "    {sign} {line}");
    }
}

pub fn show(st: &Status, diff: &[DiffEntry], new: &Since) -> String {
    let mut s = format!(
        "change: {}\nstatus: {} accepted, {} proposed, {} rejected, {} conflicts; head #{}; {}\n",
        st.intent,
        st.accepted,
        st.proposed,
        st.rejected,
        st.conflicts,
        st.head,
        age(st.cache_age_secs)
    );
    s += if diff.is_empty() {
        "diff: empty\n"
    } else {
        "diff:\n"
    };
    for e in diff {
        diff_entry(&mut s, e);
    }
    let _ = writeln!(s, "new since #{}: {}", new.from, new.entries.len());
    for e in &new.entries {
        s += &entry(e);
    }
    s
}

fn diff_entry(s: &mut String, e: &DiffEntry) {
    let _ = writeln!(s, "  {} {}", e.file, e.key);
    if e.before != e.after {
        fragment(s, '-', &e.before);
        fragment(s, '+', &e.after);
    }
    for o in &e.ops {
        let mut line = format!("    #{} {:?} {} {:?}", o.seq, o.by, o.op, o.state).to_lowercase();
        let extra = [
            ("note", &o.note),
            ("review", &o.review_note),
            ("conflict", &o.conflict),
        ];
        for (k, v) in extra {
            if let Some(v) = v {
                let _ = write!(line, "; {k}: {v}");
            }
        }
        let _ = writeln!(s, "{line}");
    }
}

fn entry(e: &Entry) -> String {
    match e {
        Entry::Op(o) => format!("  {}", record(o)),
        Entry::Review(r) => {
            let note = r.note.as_deref().unwrap_or("");
            let line = format!("  #{} {:?} {:?} op {}", r.seq, r.by, r.verdict, r.target);
            format!("{}: {note}\n", line.to_lowercase())
        }
    }
}

pub fn todo(tasks: &[Task]) -> String {
    if tasks.is_empty() {
        return "nothing to do\n".into();
    }
    let mut s = String::new();
    for t in tasks {
        let kind = format!("{:?}", t.kind).to_lowercase();
        let _ = write!(s, "- [{kind}] {}", t.text);
        if let Some(id) = &t.id {
            let _ = write!(s, " (id {id})");
        }
        s.push('\n');
        if let Some(h) = &t.help {
            let _ = writeln!(s, "    help: {h}");
        }
    }
    s
}

pub fn plan(p: &Plan) -> String {
    let mut s = String::new();
    for f in &p.files {
        let a = format!("{:?}", f.action).to_lowercase();
        let _ = writeln!(s, "{a:<6} {} ({})", f.path, f.note);
    }
    for b in &p.blockers {
        let _ = writeln!(s, "blocked: {b}");
    }
    s
}
