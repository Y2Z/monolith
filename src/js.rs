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

// Escapes anything an HTML parser could read as a closing SCRIPT tag, so that
// inlined code can't break out of the SCRIPT element it's being embedded into.
// The tag name matches ASCII case-insensitively and has to be followed by
// whitespace, a solidus, or ">" to count as an end tag (WHATWG HTML 13.2.5.22):
//   https://html.spec.whatwg.org/#script-data-end-tag-name-state
pub fn escape_script_end_tag(code: &str) -> String {
    let bytes: &[u8] = code.as_bytes();
    let mut result: String = String::with_capacity(code.len());
    let mut copied: usize = 0;
    let mut i: usize = 0;

    while i + 8 <= bytes.len() {
        if bytes[i] == b'<'
            && bytes[i + 1] == b'/'
            && bytes[i + 2..i + 8].eq_ignore_ascii_case(b"script")
            && (i + 8 == bytes.len()
                || matches!(
                    bytes[i + 8],
                    b'\t' | b'\n' | b'\x0c' | b'\r' | b' ' | b'/' | b'>'
                ))
        {
            result.push_str(&code[copied..=i]);
            result.push('\\');
            copied = i + 1;
            i += 2;
        } else {
            i += 1;
        }
    }

    result.push_str(&code[copied..]);
    result
}
