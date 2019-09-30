use crate::js::attr_is_event_handler;

#[test]
fn test_attr_is_event_handler() {
    // succeeding
    assert!(attr_is_event_handler("onBlur"));
    assert!(attr_is_event_handler("onclick"));
    assert!(attr_is_event_handler("onClick"));
    // failing
    assert!(!attr_is_event_handler("href"));
    assert!(!attr_is_event_handler(""));
    assert!(!attr_is_event_handler("class"));
}
