use crate::js::attr_is_event_handler;

#[test]
fn attr_is_event_handler() {
    // Passing
    assert!(attr_is_event_handler("onBlur"));
    assert!(attr_is_event_handler("onclick"));
    assert!(attr_is_event_handler("onClick"));

    // Failing
    assert!(!attr_is_event_handler("href"));
    assert!(!attr_is_event_handler(""));
    assert!(!attr_is_event_handler("class"));
}
