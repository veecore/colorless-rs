use std::io::{Error, ErrorKind};

pub(crate) fn is_interrupted(error: &Error) -> bool {
    matches!(error.kind(), ErrorKind::Interrupted)
}
