use std::path::Path;

pub(crate) fn refer_to_same_location(left: &Path, right: &Path) -> bool {
    match (left.canonicalize().ok(), right.canonicalize().ok()) {
        (Some(left_canonical), Some(right_canonical)) => left_canonical == right_canonical,
        _ => left == right,
    }
}

#[cfg(test)]
mod tests;
