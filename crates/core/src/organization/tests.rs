use super::*;

fn template(pattern: &str) -> OrganizationTemplate {
    OrganizationTemplate {
        pattern: pattern.into(),
        event: "Holiday".into(),
        missing_date: MissingDatePolicy::Undated,
    }
}
fn source(path: &str) -> PreviewSource {
    PreviewSource {
        id: path.into(),
        root_id: "root".into(),
        root_identity: "native-identity".into(),
        native_path: path.as_bytes().to_vec(),
        relative_path: Some(path.into()),
        display_path: path.into(),
        revision: "revision".into(),
        bytes: 42,
        capture: Some("2024-02-29T23:45:12.123".into()),
        available: true,
        companion_candidates: 1,
    }
}
fn run(pattern: &str, sources: Vec<PreviewSource>) -> OrganizationPreview {
    preview("1".into(), template(pattern), sources).unwrap()
}

#[test]
fn presets_resolve_capture_local_time_and_keep_extension_case() {
    let p = run(
        "{event}/{capture:%Y}/{capture:%Y-%m-%d}/{original_stem}.{ext}",
        vec![source("folder/港.JPG")],
    );
    assert_eq!(
        p.rows[0].target_relative_path.as_deref(),
        Some("Holiday/2024/2024-02-29/港.JPG")
    );
    assert_eq!(p.proposed, 1);
    assert!(!p.execution_available);
    assert_eq!(p.rows[0].companion_candidates, 1);
    assert_eq!(
        p.rows[0].source_components.as_deref(),
        Some(["folder".to_owned(), "港.JPG".to_owned()].as_slice())
    );
}

#[test]
fn legacy_row_serialization_stays_exact_and_source_components_follow_host_native_separators() {
    let p = run(
        "{original_stem}.{ext}",
        vec![source("Family/Trips/photo.JPG")],
    );
    let mut old = serde_json::to_value(&p.rows[0]).unwrap();
    old.as_object_mut().unwrap().remove("sourceComponents");
    let decoded: OrganizationRow = serde_json::from_value(old.clone()).unwrap();
    assert!(decoded.source_components.is_none());
    assert_eq!(serde_json::to_value(decoded).unwrap(), old);
    let p = run("{original_stem}.{ext}", vec![source("Family\\photo.JPG")]);
    #[cfg(windows)]
    assert_eq!(
        p.rows[0].source_components.as_ref().unwrap(),
        &["Family", "photo.JPG"]
    );
    #[cfg(unix)]
    assert_eq!(
        p.rows[0].source_components.as_ref().unwrap(),
        &["Family\\photo.JPG"]
    );
}

#[test]
fn rejects_general_code_unknown_tokens_and_unsafe_date_directives() {
    for pattern in [
        "",
        "/root/{ext}",
        "a//b",
        "{camera}",
        "{capture:%n}",
        "{capture:%t}",
        "{capture:%F}",
        "{capture:%}",
        "{capture:}",
        "{sequence:0}",
        "{sequence:999999999}",
        "{ext",
        "ext}",
        "{event:{ext}}",
        "x\n{ext}",
        "C:\\{ext}",
        "{capture:%Y/%m}",
    ] {
        assert!(
            preview("1".into(), template(pattern), vec![source("a.jpg")]).is_err(),
            "{pattern:?}"
        );
    }
}

#[test]
fn no_silent_filesystem_date_fallback_and_explicit_missing_value_policy() {
    let mut s = source("a.JPG");
    s.capture = None;
    let p = run("{event}/{capture:%Y}/{sequence:04}.{ext}", vec![s.clone()]);
    assert_eq!(
        p.rows[0].target_relative_path.as_deref(),
        Some("Undated/a.JPG")
    );
    assert!(p.rows[0].used_undated);
    let mut t = template("{capture:%Y}/{original_stem}.{ext}");
    t.missing_date = MissingDatePolicy::Block;
    assert_eq!(
        preview("1".into(), t, vec![s]).unwrap().rows[0].issues,
        vec![OrganizationIssue::MissingCaptureDate]
    );
    let mut t = template("{event}/{original_stem}.{ext}");
    t.event.clear();
    assert_eq!(
        preview("1".into(), t, vec![source("a.jpg")]).unwrap().rows[0].issues,
        vec![OrganizationIssue::MissingValue]
    );
}

#[test]
fn rejects_injected_values_reserved_devices_and_unsafe_components() {
    for event in [
        "..", ".", "a/b", "a\\b", "a\0b", "AUX", "NUL.txt", "COM1", "LPT¹", "a.", "a ", "x:y",
        "x\ny", "CON .jpg", "CONIN$",
    ] {
        let mut t = template("{event}/{original_stem}.{ext}");
        t.event = event.into();
        assert_eq!(
            preview("1".into(), t, vec![source("a.jpg")])
                .unwrap()
                .blocked,
            1,
            "{event:?}"
        );
    }
    assert!(
        preview(
            "1".into(),
            template("../{capture:%Y}/{original_stem}.{ext}"),
            vec![source("a.jpg")]
        )
        .is_err()
    );
    for pattern in ["NUL/{original_stem}.{ext}", "{original_stem}.png"] {
        assert_eq!(run(pattern, vec![source("a.jpg")]).blocked, 1);
    }
    let long = format!("{}.jpg", "港".repeat(90));
    assert_eq!(run("{original_stem}.{ext}", vec![source(&long)]).blocked, 1);
}

#[test]
fn stable_sequence_and_fingerprint_bind_policy_revision_and_native_identity() {
    let pattern = "{capture:%Y}/{sequence:04}_{original_stem}.{ext}";
    let a = source("a.jpg");
    let b = source("b.jpg");
    let first = run(pattern, vec![b.clone(), a.clone()]);
    let second = run(pattern, vec![a.clone(), b.clone()]);
    assert_eq!(first.fingerprint, second.fingerprint);
    assert_eq!(
        first.rows[0].target_relative_path.as_deref(),
        Some("2024/0001_a.jpg")
    );
    for changed in [
        PreviewSource {
            revision: "changed".into(),
            ..a.clone()
        },
        PreviewSource {
            root_identity: "changed".into(),
            ..a.clone()
        },
        PreviewSource {
            native_path: vec![255],
            ..a.clone()
        },
    ] {
        assert_ne!(
            first.fingerprint,
            run(pattern, vec![changed, b.clone()]).fingerprint
        );
    }
    assert_ne!(
        first.fingerprint,
        preview("2".into(), template(pattern), vec![a, b])
            .unwrap()
            .fingerprint
    );
    assert_ne!(
        first.fingerprint,
        run(
            "{sequence:04}.{ext}",
            vec![source("a.jpg"), source("b.jpg")]
        )
        .fingerprint
    );
}

#[test]
fn every_collision_participant_is_blocked_and_targets_stay_visible() {
    let p = run(
        "{original_stem}.{ext}",
        vec![
            source("a/photo.jpg"),
            source("b/photo.jpg"),
            source("c/PHOTO.jpg"),
        ],
    );
    assert_eq!(p.blocked, 3);
    assert!(p.rows.iter().all(|r| r.target_relative_path.is_some()));
    assert!(p.rows[0].issues.contains(&OrganizationIssue::Collision));
    assert!(
        p.rows[2]
            .issues
            .contains(&OrganizationIssue::PossibleCaseCollision)
    );
}

#[test]
fn undated_fallback_cannot_turn_another_proposed_file_into_a_directory() {
    let mut undated = source("photo.jpg");
    undated.capture = None;
    let p = run(
        "Undated/{original_folder}/{capture:%Y}_{original_stem}.{ext}",
        vec![undated, source("photo.jpg/a.jpg")],
    );
    assert_eq!(p.blocked, 2);
    assert!(
        p.rows
            .iter()
            .all(|row| row.issues.contains(&OrganizationIssue::Collision))
    );
}

#[test]
fn missing_offline_and_lossy_paths_are_not_ready_sources() {
    let mut s = source("a.jpg");
    s.available = false;
    let mut bad = source("b.jpg");
    bad.relative_path = None;
    let p = run("{original_stem}.{ext}", vec![s, bad]);
    assert_eq!(p.blocked, 2);
    assert_eq!(p.rows[0].issues, vec![OrganizationIssue::SourceUnavailable]);
    assert_eq!(p.rows[1].issues, vec![OrganizationIssue::NonUnicodePath]);
}

#[test]
fn bounded_inputs_and_nonlossy_byte_totals() {
    assert!(preview("1".into(), template("{ext}"), vec![]).is_err());
    assert!(
        preview(
            "1".into(),
            template("{ext}"),
            vec![source("a.jpg"); PREVIEW_LIMIT + 1]
        )
        .is_err()
    );
    let mut s = source("a.jpg");
    s.bytes = u64::MAX;
    assert_eq!(
        run("{original_stem}.{ext}", vec![s.clone(), s]).selected_bytes,
        "36893488147419103230"
    );
}
#[test]
fn template_examples_use_the_preview_resolver_without_filesystem_access() {
    use super::{MissingDatePolicy, OrganizationTemplate, TemplateSample, validate_samples};
    let mut template = OrganizationTemplate {
        pattern: "{capture:%Y}/{original_folder}/{sequence:03}_{original_stem}.{ext}".into(),
        event: String::new(),
        missing_date: MissingDatePolicy::Undated,
    };
    let mut sample = TemplateSample {
        relative_path: "Holiday/IMG.JPG".into(),
        capture: Some("2026-10-31T14:30:00.123".into()),
        sequence: 12,
        expected_target: "2026/Holiday/012_IMG.JPG".into(),
    };
    validate_samples(&template, &[sample.clone()]).unwrap();
    sample.expected_target = "2026/Holiday/012_IMG.jpg".into();
    assert!(validate_samples(&template, &[sample.clone()]).is_err());
    sample.capture = None;
    sample.expected_target = "Undated/IMG.JPG".into();
    validate_samples(&template, &[sample.clone()]).unwrap();
    template.missing_date = MissingDatePolicy::Block;
    assert!(validate_samples(&template, &[sample.clone()]).is_err());
    template.pattern = "{original_stem}.{ext}".into();
    sample.expected_target = "IMG.JPG".into();
    for path in [
        "../IMG.JPG",
        "C:/IMG.JPG",
        "a\\IMG.JPG",
        "/IMG.JPG",
        "a//IMG.JPG",
        "NUL/IMG.JPG",
    ] {
        sample.relative_path = path.into();
        assert!(
            validate_samples(&template, &[sample.clone()]).is_err(),
            "{path}"
        );
    }
    sample.relative_path = "Holiday/IMG.JPG".into();
    for date in [
        "2026-02-30T12:00:00",
        "2026-10-31T14:30:00Z",
        "",
        "not a date",
    ] {
        sample.capture = Some(date.into());
        assert!(validate_samples(&template, &[sample.clone()]).is_err());
    }
    sample.capture = None;
    for sequence in [0, 241, u32::MAX] {
        sample.sequence = sequence;
        assert!(validate_samples(&template, &[sample.clone()]).is_err());
    }
    sample.sequence = 1;
    validate_samples(&template, &vec![sample.clone(); 4]).unwrap();
    assert!(validate_samples(&template, &vec![sample; 5]).is_err());
}
