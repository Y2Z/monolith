extern crate html5ever;
#[macro_use]
extern crate lazy_static;
extern crate regex;
extern crate reqwest;
extern crate url;

#[macro_use]
mod macros;

pub mod html;
pub mod http;
pub mod js;
pub mod utils;

#[cfg(test)]
pub mod tests;
