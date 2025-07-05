//! Override parsing functionality
//!
//! This module contains parsers for command override syntax

pub mod override_parser;
pub mod smart_parser;

pub use override_parser::{
    OptionValue, OverrideParser, ParsedOverride, parse_override, parse_override_to_command,
};
pub use smart_parser::{OverrideOperator, ParsedOverrides, SmartOverrideParser};
