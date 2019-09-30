const JS_DOM_EVENT_ATTRS: [&str; 21] = [
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
    JS_DOM_EVENT_ATTRS.contains(&attr_name.to_lowercase().as_str())
}
