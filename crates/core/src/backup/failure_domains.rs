//! Fresh native physical-storage observations. Never deserialize these as proof.
//! Distinct volumes or successful localhost transports do not prove independence.
use crate::domain::Result;
use std::{collections::BTreeSet, fs::File};
#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;

pub(crate) fn local(file: &File) -> Result<Option<BTreeSet<String>>> {
    #[cfg(windows)]
    {
        windows::local(file)
    }
    #[cfg(target_os = "macos")]
    {
        macos::local(file)
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let _ = file;
        Ok(None)
    }
}
pub(crate) fn independent_local(source: &File, repository: &File) -> Result<bool> {
    let a = local(source)?;
    let b = local(repository)?;
    if !disjoint(a.as_ref(), b.as_ref()) {
        return Ok(false);
    }
    // Disk/registry IDs are ephemeral observations. A changed observation cannot
    // be promoted into a reusable persisted approval or failure-domain claim.
    Ok(local(source)? == a && local(repository)? == b)
}
fn disjoint(a: Option<&BTreeSet<String>>, b: Option<&BTreeSet<String>>) -> bool {
    matches!((a,b),(Some(a),Some(b)) if !a.is_empty() && !b.is_empty() && a.is_disjoint(b))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn partitions_unknown_and_shared_multidisk_domains_are_not_independent() {
        let a = BTreeSet::from(["physical:0".into()]);
        let same = a.clone();
        let b = BTreeSet::from(["physical:1".into()]);
        let spanning = BTreeSet::from(["physical:0".into(), "physical:1".into()]);
        assert!(!disjoint(Some(&a), Some(&same)));
        assert!(!disjoint(Some(&a), None));
        assert!(!disjoint(None, Some(&b)));
        assert!(!disjoint(Some(&a), Some(&spanning)));
        assert!(!disjoint(Some(&BTreeSet::new()), Some(&b)));
        assert!(disjoint(Some(&a), Some(&b)));
    }
    #[test]
    fn native_owned_directory_never_proves_independence_from_itself() {
        let owned = tempfile::tempdir().unwrap();
        let directory = crate::filesystem::open_directory_readonly(owned.path()).unwrap();
        assert!(!independent_local(&directory, &directory).unwrap());
    }
}
