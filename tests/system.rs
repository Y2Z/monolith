//! Tests that run real browsers: pages get saved using -B, and saved documents get opened by the
//! same browsers afterwards, with a local server keeping track of everything that reaches it.
//!
//! Every browser that can be found in PATH gets used, one of each kind, or whatever's listed in
//! MONOLITH_TEST_BROWSERS. Nothing gets tested if there are none, unless
//! MONOLITH_TEST_REQUIRE_BROWSERS is set: that's when both kinds of browsers have to be there.
//!
//! Cargo runs test targets in alphabetical order, this name keeps these after the unit tests.

#![cfg(feature = "browser")]

#[path = "system/support.rs"]
mod support;

#[path = "system/capture.rs"]
mod capture;
#[path = "system/csp.rs"]
mod csp;
#[path = "system/page.rs"]
mod page;
#[path = "system/privacy.rs"]
mod privacy;
#[path = "system/replay.rs"]
mod replay;

#[test]
fn has_browsers_to_test() {
    if std::env::var_os("MONOLITH_TEST_REQUIRE_BROWSERS").is_none() {
        return;
    }

    use monolith::browser::uses_devtools;

    let browsers = support::browsers();
    assert!(
        browsers.iter().any(|path| uses_devtools(path)),
        "no Chromium-based browser in {browsers:?}"
    );
    assert!(
        browsers.iter().any(|path| !uses_devtools(path)),
        "no Firefox-based browser in {browsers:?}"
    );
}
