//! Change writes answer the change and the touched preview levels (in-place patching):
//! planned edges, rules without a reason, retract, re-apply, link check, levels, selectors.

use super::*;

async fn json(s: &Site, path: &str, body: Value) -> Value {
    let (st, text) = s.post(path, body).await;
    assert_eq!(st, StatusCode::OK, "{path}: {text}");
    serde_json::from_str(&text).unwrap()
}

fn kinds(v: &Value) -> Vec<String> {
    let todo = v["change"]["todo"].as_array().unwrap();
    todo.iter()
        .map(|t| t["kind"].as_str().unwrap().to_string())
        .collect()
}

#[tokio::test]
async fn writes_answer_the_change_and_the_touched_levels() {
    let s = Site::new();
    json(&s, "/api/change/open", json!({"intent": "x"})).await;
    let add = json!({"op": {"op": "add_brick", "file": "brickie.toml",
        "brick": {"name": "cache", "kind": "crate"}}});
    let r = json(&s, "/api/change/op", add).await;
    assert_eq!(r["done"]["seq"], json!(1));
    assert_eq!(r["levels"][0]["path"], "root");
    let root = &r["change"]["levels"]["root"];
    assert_eq!(
        root["selectors"],
        json!(["*", "engine", "cache", "tag:core"])
    );
    assert_eq!(root["tags"], json!(["core"]));

    let rule = json!({"op": {"op": "add_rule", "file": "brickie.toml",
        "rule": {"from": "cache", "to": "engine", "action": "allow"}}});
    let r = json(&s, "/api/change/op", rule).await;
    let edges = r["levels"][0]["planned_edges"].as_array().unwrap();
    assert_eq!(edges.len(), 1, "{r}");
    assert_eq!(
        (
            edges[0]["from"].clone(),
            edges[0]["rule"].clone(),
            edges[0]["by"].clone()
        ),
        (json!("cache"), json!(0), json!("human"))
    );
    assert_eq!(r["change"]["planned_edges"].as_array().unwrap().len(), 1);
    let k = kinds(&r);
    assert!(
        k.contains(&"link".into()) && k.contains(&"reason".into()),
        "{k:?}"
    );
    let plan: Value = serde_json::from_str(&s.get("/api/change/plan").await.1).unwrap();
    assert_eq!(plan["warnings"].as_array().unwrap().len(), 1, "{plan}");

    let check = json!({"from": "cache", "to": "engine", "scope": "prod", "action": "allow"});
    let c = json(&s, "/api/change/link-check", check).await;
    assert_eq!(c["redundant"], json!(true), "{c}");
    let check = json!({"from": "engine", "to": "cache", "scope": "all", "action": "deny"});
    let c = json(&s, "/api/change/link-check", check).await;
    assert_eq!(c["redundant"], json!(false), "{c}");

    let lv = json(&s, "/api/change/levels", json!({"paths": ["root"]})).await;
    assert_eq!(lv.as_array().unwrap().len(), 1);

    let r = json(&s, "/api/change/retract", json!({})).await;
    assert_eq!(
        (r["done"]["target"].clone(), r["done"]["verdict"].clone()),
        (json!(2), json!("retract"))
    );
    assert!(r["levels"][0].get("planned_edges").is_none(), "{r}");
    let (st, _) = s.post("/api/change/reapply", json!({"seq": 1})).await;
    assert_eq!(st, StatusCode::CONFLICT);
}
