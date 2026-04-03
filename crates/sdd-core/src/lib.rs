pub mod coverage;
pub mod error;
pub mod models;
pub mod parser;
pub mod scanner;

pub use coverage::{compute_coverage, compute_stats, extract_type_from_id};
pub use error::{CoreError, Result};
pub use models::*;
pub use parser::{parse_requirements, parse_tasks};
pub use scanner::{is_test_file, scan_directory, scan_file, supported_extensions};
