const JS_DOM_EVENT_ATTRS: &[&str] = &[
    // Input
    "onfocus",
    "onblur",
    "onselect",
    "onchange",
    "onsubmit",
    "onreset",
    "onkeydown",
    "onkeypress",
    "onkeyup",
    // Mouse
    "onmouseover",
    "onmouseout",
    "onmousedown",
    "onmouseup",
    "onmousemove",
    // Click
    "onclick",
    "ondblclick",
    // Load
    "onload",
    "onunload",
    "onabort",
    "onerror",
    "onresize",
];

// Returns true if DOM attribute name matches a native JavaScript event handler
pub fn attr_is_event_handler(attr_name: &str) -> bool {
    JS_DOM_EVENT_ATTRS
        .iter()
        .find(|a| attr_name.eq_ignore_ascii_case(a))
        .is_some()
}
