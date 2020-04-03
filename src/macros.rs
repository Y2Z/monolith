#[macro_export]
macro_rules! str {
    () => {
        String::new()
    };
    ($val: expr) => {
        ToString::to_string(&$val)
    };
}

#[macro_export]
macro_rules! empty_image {
    () => {
"data:image/png;base64,\
iVBORw0KGgoAAAANSUhEUgAAAA0AAAANCAQAAADY4iz3AAAAEUlEQVR42mNkwAkYR6UolgIACvgADsuK6xYAAAAASUVORK5CYII="
    };
}
