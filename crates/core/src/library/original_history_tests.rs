use super::*;
use crate::{
    filesystem,
    library::{
        model::{RootMode, ScanFilter},
        native, store,
    },
};
use std::sync::Arc;

struct Case {
    catalog: Arc<Catalog>,
    run: tempfile::TempDir,
    job: String,
    plan: rename_graph::Plan,
}
impl Case {
    fn new() -> Self {
        let run = tempfile::Builder::new()
            .prefix("stillharbour-original-history-")
            .tempdir()
            .unwrap();
        let source = run.path().join("source");
        std::fs::create_dir(&source).unwrap();
        let plan: rename_graph::Plan = serde_json::from_value(
            rename_graph::fixture_preview_probe().unwrap()["forward"].clone(),
        )
        .unwrap();
        for member in &plan.members {
            std::fs::write(source.join(&member.before), member.id.as_bytes()).unwrap();
        }
        let catalog = Arc::new(Catalog::open(run.path().join("History.stillharbour")).unwrap());
        let directory = native::open_root(&source, None).unwrap();
        let identity = filesystem::identity(&directory).unwrap();
        let volume = native::volume_token(&directory).unwrap();
        let root = store::add_root(
            &catalog,
            source,
            identity.clone(),
            volume,
            "Generated history fixture".into(),
            RootMode::Managed,
            ScanFilter::default(),
        )
        .unwrap();
        super::super::original_jobs::retain_plan(
            &catalog,
            root,
            (identity.volume, identity.file),
            plan.clone(),
            None,
        )
        .unwrap();
        let job = super::super::original_jobs::create(&catalog, plan.id.clone())
            .unwrap()
            .id;
        Self {
            run,
            catalog,
            job,
            plan,
        }
    }
    fn unchanged(&self) {
        for member in &self.plan.members {
            assert_eq!(
                std::fs::read(self.run.path().join("source").join(&member.before)).unwrap(),
                member.id.as_bytes()
            );
        }
        assert_eq!(
            std::fs::read_dir(self.run.path().join("source"))
                .unwrap()
                .count(),
            self.plan.members.len()
        );
    }
}

#[test]
fn original_history_is_bounded_read_only_and_does_not_reauthorize_recovered_jobs() {
    let case = Case::new();
    super::super::original_jobs::recover(&case.catalog).unwrap();
    case.catalog
        .call(|db| {
            db.execute("UPDATE roots SET availability='offline'", [])?;
            db.pragma_update(None, "query_only", true)?;
            Ok(())
        })
        .unwrap();
    let count = || {
        case.catalog
            .call(|db| {
                Ok(db.query_row(
                    "SELECT count(*) FROM original_operation_events",
                    [],
                    |row| row.get::<_, u32>(0),
                )?)
            })
            .unwrap()
    };
    let before = count();
    for _ in 0..2 {
        let history = review(&case.catalog, case.job.clone()).unwrap();
        assert_eq!(history.job.state, OriginalJobState::NeedsRecovery);
        assert!(history.history_only);
        assert!(!history.execution_available);
        assert!(!history.current_locations_checked);
        assert_eq!(history.members.len(), case.plan.members.len());
        for (actual, planned) in history.members.iter().zip(&case.plan.members) {
            assert_eq!(
                (&actual.id, &actual.before, &actual.after),
                (&planned.id, &planned.before, &planned.after)
            );
        }
    }
    assert_eq!(count(), before);
    assert!(review(&case.catalog, uuid::Uuid::new_v4().to_string()).is_err());
    assert!(review(&case.catalog, "../not-a-job".into()).is_err());
    case.unchanged();
}

#[test]
fn original_history_rejects_changed_imported_records_even_with_recomputed_payload_hash() {
    for variant in 0..9 {
        let case = Case::new();
        case.catalog
            .call(move |db| {
                let (id, payload): (String, String) = db.query_row(
                    "SELECT id,payload FROM original_operation_plans",
                    [],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )?;
                let mut value: serde_json::Value = serde_json::from_str(&payload).unwrap();
                match variant {
                    0 => value["unexpectedApproval"] = serde_json::json!(true),
                    1 => value["catalogId"] = serde_json::json!(uuid::Uuid::new_v4().to_string()),
                    2 => value["rootId"] = serde_json::json!(uuid::Uuid::new_v4().to_string()),
                    3 => value["graph"]["execution_available"] = serde_json::json!(true),
                    4 => value["reverseOf"] = serde_json::json!(uuid::Uuid::new_v4().to_string()),
                    5 => value["sourceDevice"] = serde_json::json!("00"),
                    6 => value["graph"]["steps"][0]["to"] = serde_json::json!("unreviewed.JPG"),
                    7 => value["graph"]["policy"] = serde_json::json!("unknown-policy"),
                    _ => {}
                }
                let payload = serde_json::to_string(&value).unwrap();
                let digest = if variant == 8 {
                    "0".repeat(64)
                } else {
                    blake3::hash(payload.as_bytes()).to_hex().to_string()
                };
                // Imported database records are untrusted. Drop only the immutable
                // trigger in this owned fixture to emulate a modified catalog.
                db.execute_batch("DROP TRIGGER original_plan_no_update")?;
                db.execute(
                    "UPDATE original_operation_plans SET payload=?,digest=? WHERE id=?",
                    rusqlite::params![payload, digest, id],
                )?;
                Ok(())
            })
            .unwrap();
        assert!(
            review(&case.catalog, case.job.clone()).is_err(),
            "variant {variant}"
        );
        case.unchanged();
    }
}
