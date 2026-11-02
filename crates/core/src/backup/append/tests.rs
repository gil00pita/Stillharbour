use super::*;
use crate::backup::comparison::{ComparisonEntryKind, SyncProposal};

fn row(name: &str, category: Category) -> ComparisonEntry {
    let source = category != Category::DestinationOnly;
    let destination = category != Category::SourceOnly;
    ComparisonEntry {
        companions: vec![],
        source_kind: source.then_some(ComparisonEntryKind::File),
        destination_kind: destination.then_some(ComparisonEntryKind::File),
        id: blake3::hash(name.as_bytes()).to_hex().to_string(),
        display_path: name.into(),
        source_bytes: source.then(|| "7".into()),
        destination_bytes: destination.then(|| "7".into()),
        source_modified: None,
        destination_modified: None,
        category,
        proposal: SyncProposal::CopyAfterReview,
        evidence: String::new(),
    }
}
fn review(rows: &[ComparisonEntry]) -> AppendReview<'_> {
    AppendReview {
        run_id: "run",
        comparison_digest: Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
        native_scope_digest: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        mode: CompareMode::Verified,
        state: ComparisonState::NeedsReview,
        rows,
    }
}

#[test]
fn append_plan_retains_destination_and_requires_explicit_difference_decisions() {
    let rows = vec![
        row("new.mp4", Category::SourceOnly),
        row("only.xmp", Category::DestinationOnly),
        row("same", Category::VerifiedEqual),
        row("edited", Category::DestinationEditedSinceLastRun),
    ];
    assert!(plan(review(&rows), &[]).is_err());
    let result = plan(review(&rows), &[rows[3].id.clone()]).unwrap();
    assert_eq!(result.copy_files, 1);
    assert_eq!(result.copy_bytes, "7");
    assert_eq!(result.retained_destination_files, 2);
    assert_eq!(result.uncopied_source_revisions, 1);
    assert!(!result.execution_available);
    assert_eq!(
        result
            .items
            .iter()
            .filter(|r| r.action == AppendAction::CopyNew)
            .count(),
        1
    );
}

#[test]
fn append_plan_rejects_quick_partial_canceled_failed_or_missing_evidence() {
    let rows = vec![row("new", Category::SourceOnly)];
    for state in [
        ComparisonState::Comparing,
        ComparisonState::Incomplete,
        ComparisonState::Canceled,
        ComparisonState::Failed,
    ] {
        let mut r = review(&rows);
        r.state = state;
        assert!(plan(r, &[]).is_err());
    }
    let mut r = review(&rows);
    r.mode = CompareMode::Quick;
    assert!(plan(r, &[]).is_err());
    let mut r = review(&rows);
    r.comparison_digest = None;
    assert!(plan(r, &[]).is_err());
    let mut r = review(&rows);
    r.native_scope_digest = "foreign";
    assert!(plan(r, &[]).is_err());
}

#[test]
fn append_plan_never_turns_unknown_blocked_or_quick_rows_into_copy() {
    for category in [
        Category::Blocked,
        Category::Uncertain,
        Category::AppearsUnchangedUnverified,
    ] {
        let rows = vec![row("file", category)];
        assert!(plan(review(&rows), &[]).is_err());
        assert!(plan(review(&rows), &[rows[0].id.clone()]).is_err());
    }
}

#[test]
fn append_plan_rejects_foreign_repeated_and_inapplicable_decisions() {
    let rows = vec![
        row("new", Category::SourceOnly),
        row("edit", Category::DifferentContent),
    ];
    for decisions in [
        vec!["foreign".into()],
        vec![rows[1].id.clone(), rows[1].id.clone()],
        vec![rows[0].id.clone()],
    ] {
        assert!(plan(review(&rows), &decisions).is_err());
    }
    assert!(plan(review(&[rows[0].clone(), rows[0].clone()]), &[]).is_err());
}

#[test]
fn append_plan_checks_native_object_kinds_and_lengths() {
    for category in [
        Category::SourceOnly,
        Category::DestinationOnly,
        Category::VerifiedEqual,
        Category::DifferentContent,
    ] {
        let mut rows = vec![row("file", category)];
        rows[0].destination_kind = Some(ComparisonEntryKind::Unsupported);
        assert!(plan(review(&rows), &[rows[0].id.clone()]).is_err());
        assert!(plan(review(&rows), &[]).is_err());
    }
    for length in [None, Some("-1"), Some("18446744073709551616")] {
        let mut rows = vec![row("new", Category::SourceOnly)];
        rows[0].source_bytes = length.map(String::from);
        assert!(plan(review(&rows), &[]).is_err());
    }
}

#[test]
fn append_plan_covers_all_pages_and_sums_without_u64_overflow() {
    let mut rows: Vec<_> = (0..410)
        .map(|i| row(&format!("{i}.raw"), Category::SourceOnly))
        .collect();
    rows[0].source_bytes = Some(u64::MAX.to_string());
    rows[409].source_bytes = Some(u64::MAX.to_string());
    let result = plan(review(&rows), &[]).unwrap();
    assert_eq!(result.copy_files, 410);
    assert_eq!(
        result.copy_bytes,
        (2 * u128::from(u64::MAX) + 408 * 7).to_string()
    );
    rows[409].category = Category::Uncertain;
    assert!(plan(review(&rows), &[]).is_err());
}

#[test]
fn append_plan_digest_binds_scope_decisions_and_run_but_not_row_order() {
    let rows = vec![
        row("first", Category::SourceOnly),
        row("last", Category::DestinationOnly),
    ];
    let a = plan(review(&rows), &[]).unwrap();
    let mut reversed = rows.clone();
    reversed.reverse();
    assert_eq!(a.digest, plan(review(&reversed), &[]).unwrap().digest);
    let mut r = review(&rows);
    r.run_id = "another";
    assert_ne!(a.digest, plan(r, &[]).unwrap().digest);
    let mut r = review(&rows);
    r.native_scope_digest = r.comparison_digest.unwrap();
    assert_ne!(a.digest, plan(r, &[]).unwrap().digest);
    let mut r = review(&rows);
    r.comparison_digest = Some(r.native_scope_digest);
    assert_ne!(a.digest, plan(r, &[]).unwrap().digest);
    let mut changed = rows;
    changed[0].source_bytes = Some("8".into());
    assert_ne!(a.digest, plan(review(&changed), &[]).unwrap().digest);
}

#[test]
fn append_plan_bounds_oversized_reports_and_retains_empty_scope_without_success_claim() {
    assert!(
        plan(
            review(&vec![row("same", Category::SourceOnly); 20_001]),
            &[]
        )
        .is_err()
    );
    let result = plan(review(&[]), &[]).unwrap();
    assert_eq!(result.copy_files, 0);
    assert!(!result.execution_available);
}
