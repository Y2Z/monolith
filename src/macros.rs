#[macro_export]
macro_rules! str {
    () => {
        String::new()
    };
    ($val: expr) => {
        ToString::to_string(&$val)
    };
}
