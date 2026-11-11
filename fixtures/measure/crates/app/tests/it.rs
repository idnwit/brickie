#[test]
fn integration() {
    let s = mx_app::model::Square(2);
    assert_eq!(mx_app::api::run(&s), 4);
}
