const JS_DOM_EVENT_ATTRS: &[&str] = &[
    // From WHATWG HTML spec 8.1.5.2 "Event handlers on elements, Document objects, and Window objects":
    //   https://html.spec.whatwg.org/#event-handlers-on-elements,-document-objects,-and-window-objects
    //   https://html.spec.whatwg.org/#attributes-3 (table "List of event handler content attributes")

    // Global event handlers
    "onabort",
    "onauxclick",
    "onblur",
    "oncancel",
    "oncanplay",
    "oncanplaythrough",
    "onchange",
    "onclick",
    "onclose",
    "oncontextmenu",
    "oncuechange",
    "ondblclick",
    "ondrag",
    "ondragend",
    "ondragenter",
    "ondragexit",
    "ondragleave",
    "ondragover",
    "ondragstart",
    "ondrop",
    "ondurationchange",
    "onemptied",
    "onended",
    "onerror",
    "onfocus",
    "onformdata",
    "oninput",
    "oninvalid",
    "onkeydown",
    "onkeypress",
    "onkeyup",
    "onload",
    "onloadeddata",
    "onloadedmetadata",
    "onloadstart",
    "onmousedown",
    "onmouseenter",
    "onmouseleave",
    "onmousemove",
    "onmouseout",
    "onmouseover",
    "onmouseup",
    "onwheel",
    "onpause",
    "onplay",
    "onplaying",
    "onprogress",
    "onratechange",
    "onreset",
    "onresize",
    "onscroll",
    "onsecuritypolicyviolation",
    "onseeked",
    "onseeking",
    "onselect",
    "onslotchange",
    "onstalled",
    "onsubmit",
    "onsuspend",
    "ontimeupdate",
    "ontoggle",
    "onvolumechange",
    "onwaiting",
    "onwebkitanimationend",
    "onwebkitanimationiteration",
    "onwebkitanimationstart",
    "onwebkittransitionend",
    // Event handlers for <body/> and <frameset/> elements
    "onafterprint",
    "onbeforeprint",
    "onbeforeunload",
    "onhashchange",
    "onlanguagechange",
    "onmessage",
    "onmessageerror",
    "onoffline",
    "ononline",
    "onpagehide",
    "onpageshow",
    "onpopstate",
    "onrejectionhandled",
    "onstorage",
    "onunhandledrejection",
    "onunload",
    // Event handlers for <html/> element
    "oncut",
    "oncopy",
    "onpaste",
];

// Returns true if DOM attribute name matches a native JavaScript event handler
pub fn attr_is_event_handler(attr_name: &str) -> bool {
    JS_DOM_EVENT_ATTRS
        .iter()
        .any(|a| attr_name.eq_ignore_ascii_case(a))
}

// Neutralizes the only two sequences that let script content affect the
// HTML tokenizer: "</script" (ends the element) and "<!--" (the way into the
// double-escaped state, where the real end tag stops working).
// https://html.spec.whatwg.org/#restrictions-for-contents-of-script-elements
pub fn escape_script_end_tag(code: &str) -> String {
    let bytes = code.as_bytes();
    let mut result = String::with_capacity(code.len());
    let mut copied = 0;

    for (i, _) in code.match_indices('<') {
        let rest = &bytes[i + 1..];
        let end_tag =
            rest.len() >= 7 && rest[0] == b'/' && rest[1..7].eq_ignore_ascii_case(b"script");
        if end_tag || rest.starts_with(b"!--") {
            result.push_str(&code[copied..=i]);
            result.push('\\');
            copied = i + 1;
        }
    }

    result.push_str(&code[copied..]);
    result
}
