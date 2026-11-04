// SPDX-License-Identifier: GPL-3.0-or-later

//! Structural invariants over a single compilation database.
//!
//! Stage 4 of the dogfooding plan needs baseline-free checks that a captured
//! database is internally well-formed: no entry has an empty argument vector,
//! no two entries are true duplicates, and (opt-in) the entry count lands in an
//! expected band. The decision logic lives here with unit tests rather than in
//! shell, because the duplicate check must reuse the normalization and get the
//! legitimate multi-output case right (the Stage 3 lesson).
//!
//! The checks run over an already-normalized [`CompilationDatabase`]: the caller
//! applies [`crate::cdb::Normalization`] first, so the duplicate key uses
//! normalized arguments automatically.

use serde::Serialize;
use std::collections::HashMap;

use crate::cdb::CompilationDatabase;

/// The outcome of a single invariant check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Pass,
    Fail,
    Skipped,
}

/// One offending entry pointed at by a failing check. The optional fields are
/// omitted when absent, so a `non-empty-arguments` offender serializes as just
/// `{"file": "..."}` while a duplicate offender carries `output` and `count`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Offender {
    pub file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<usize>,
}

/// The numeric detail attached to the entry-count check, describing what was
/// asserted. Only the fields in play for the given flags are emitted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CountDetail {
    pub entries: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tolerance_pct: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<usize>,
}

/// A single named check in the report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Check {
    pub name: String,
    pub status: Status,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub offenders: Vec<Offender>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<CountDetail>,
}

/// The full invariants report for one database.
#[derive(Debug, Serialize)]
pub struct InvariantsReport {
    pub pass: bool,
    pub checks: Vec<Check>,
}

impl InvariantsReport {
    /// Render the report as one readable line per check, with offenders or detail
    /// indented beneath a failing check so a failure localizes.
    pub fn to_human(&self) -> String {
        let mut out = String::new();
        out.push_str(if self.pass { "invariants: pass\n" } else { "invariants: fail\n" });
        for check in &self.checks {
            let status = match check.status {
                Status::Pass => "pass",
                Status::Fail => "fail",
                Status::Skipped => "skipped",
            };
            out.push_str(&format!("  {} - {}\n", check.name, status));
            for offender in &check.offenders {
                let mut line = format!("    {}", offender.file);
                if let Some(output) = &offender.output {
                    line.push_str(&format!(" -> {output}"));
                }
                if let Some(count) = offender.count {
                    line.push_str(&format!(" (x{count})"));
                }
                line.push('\n');
                out.push_str(&line);
            }
            if let Some(detail) = &check.detail {
                let mut line = format!("    entries={}", detail.entries);
                if let Some(expected) = detail.expected {
                    line.push_str(&format!(" expected={expected}"));
                }
                if let Some(tolerance) = detail.tolerance_pct {
                    line.push_str(&format!(" tolerance_pct={tolerance}"));
                }
                if let Some(min) = detail.min {
                    line.push_str(&format!(" min={min}"));
                }
                line.push('\n');
                out.push_str(&line);
            }
        }
        out
    }
}

/// The opt-in constraints for the entry-count check. When both are absent the
/// check is reported as skipped.
#[derive(Debug, Clone, Copy, Default)]
pub struct CountExpectation {
    /// Expected object count; the band is `+/- ceil(N * tolerance_pct / 100)`.
    pub expected_objects: Option<usize>,
    /// Tolerance percent for the expected-objects band; defaults to 0.
    pub tolerance_pct: usize,
    /// A simple floor: assert `entries >= min`.
    pub min_entries: Option<usize>,
}

impl CountExpectation {
    fn is_requested(&self) -> bool {
        self.expected_objects.is_some() || self.min_entries.is_some()
    }
}

/// Run the structural invariants over an already-normalized database.
///
/// Deferred refinement: exact object-set membership (assert each produced `.o`
/// has a matching entry) is not built here; the entry-count band approximates
/// "every translation unit appears" for this MVP.
pub fn check(database: &CompilationDatabase, expectation: &CountExpectation) -> InvariantsReport {
    let checks = vec![
        non_empty_arguments(database),
        non_empty_directory(database),
        no_true_duplicates(database),
        entry_count(database, expectation),
    ];
    let pass = checks.iter().all(|check| check.status != Status::Fail);
    InvariantsReport { pass, checks }
}

/// Every entry must carry a non-empty `arguments` vector.
fn non_empty_arguments(database: &CompilationDatabase) -> Check {
    let offenders: Vec<Offender> = database
        .entries
        .iter()
        .filter(|entry| entry.arguments.is_empty())
        .map(|entry| Offender { file: entry.file.clone(), output: None, count: None })
        .collect();
    let status = if offenders.is_empty() { Status::Pass } else { Status::Fail };
    Check { name: "non-empty-arguments".to_string(), status, offenders, detail: None }
}

/// Every entry must carry a non-empty `directory`. A blank `directory` makes the
/// recorded command non-replayable: a consumer that does `cd "$directory"` lands
/// in an unrelated tree, and the replay loop's `cd ""` is a silent no-op that
/// would validate the entry against the wrong working directory.
fn non_empty_directory(database: &CompilationDatabase) -> Check {
    let offenders: Vec<Offender> = database
        .entries
        .iter()
        .filter(|entry| entry.directory.is_empty())
        .map(|entry| Offender { file: entry.file.clone(), output: None, count: None })
        .collect();
    let status = if offenders.is_empty() { Status::Pass } else { Status::Fail };
    Check { name: "non-empty-directory".to_string(), status, offenders, detail: None }
}

/// No two entries may be true duplicates. Entries are grouped by the full triple
/// `(file, output, arguments)`; any group with two or more members is a true
/// duplicate. Grouping by the full triple (not just `(file, output)`) is what
/// keeps the legitimate multi-output case - the same source compiled into
/// different objects with different flags - in distinct groups, so it is not
/// flagged; it also produces the reported `{file, output, count}` shape.
fn no_true_duplicates(database: &CompilationDatabase) -> Check {
    type Key<'a> = (&'a str, Option<&'a str>, &'a [String]);

    let mut groups: HashMap<Key, usize> = HashMap::new();
    // Preserve first-seen order so the offender list is deterministic.
    let mut order: Vec<Key> = Vec::new();
    for entry in &database.entries {
        let key: Key = (entry.file.as_str(), entry.output.as_deref(), entry.arguments.as_slice());
        let count = groups.entry(key).or_insert_with(|| {
            order.push(key);
            0
        });
        *count += 1;
    }

    let offenders: Vec<Offender> = order
        .into_iter()
        .filter_map(|key| {
            let count = groups[&key];
            (count >= 2).then(|| Offender {
                file: key.0.to_string(),
                output: key.1.map(str::to_string),
                count: Some(count),
            })
        })
        .collect();
    let status = if offenders.is_empty() { Status::Pass } else { Status::Fail };
    Check { name: "no-true-duplicates".to_string(), status, offenders, detail: None }
}

/// Opt-in entry-count check. Skipped unless `--expected-objects` or
/// `--min-entries` is given. When requested it passes iff every provided
/// constraint holds (both, when both are given).
fn entry_count(database: &CompilationDatabase, expectation: &CountExpectation) -> Check {
    let name = "entry-count".to_string();
    if !expectation.is_requested() {
        return Check { name, status: Status::Skipped, offenders: Vec::new(), detail: None };
    }

    let entries = database.entries.len();
    let mut holds = true;
    let mut detail = CountDetail { entries, expected: None, tolerance_pct: None, min: None };

    if let Some(expected) = expectation.expected_objects {
        // Integer band: |entries - expected| <= ceil(expected * pct / 100).
        // Saturating arithmetic throughout so an extreme expected/tolerance
        // pair cannot overflow (this is a test tool fed arbitrary numbers).
        let band = expected.saturating_mul(expectation.tolerance_pct).div_ceil(100);
        let low = expected.saturating_sub(band);
        let high = expected.saturating_add(band);
        holds &= entries >= low && entries <= high;
        detail.expected = Some(expected);
        detail.tolerance_pct = Some(expectation.tolerance_pct);
    }
    if let Some(min) = expectation.min_entries {
        holds &= entries >= min;
        detail.min = Some(min);
    }

    let status = if holds { Status::Pass } else { Status::Fail };
    Check { name, status, offenders: Vec::new(), detail: Some(detail) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cdb::{Entry, Normalization};

    /// A self-consistent entry with the given file, output, and arguments.
    fn entry(file: &str, output: Option<&str>, arguments: &[&str]) -> Entry {
        Entry {
            directory: "/build".to_string(),
            file: file.to_string(),
            arguments: arguments.iter().map(|s| s.to_string()).collect(),
            output: output.map(str::to_string),
        }
    }

    fn database(entries: Vec<Entry>) -> CompilationDatabase {
        CompilationDatabase { entries }
    }

    /// Find a check by name in a report.
    fn find<'a>(report: &'a InvariantsReport, name: &str) -> &'a Check {
        report.checks.iter().find(|check| check.name == name).expect("check present")
    }

    #[test]
    fn non_empty_arguments_fails_on_empty_and_passes_otherwise() {
        let cases: &[(Vec<Entry>, Status)] = &[
            (vec![entry("/src/a.c", Some("a.o"), &[])], Status::Fail),
            (vec![entry("/src/a.c", Some("a.o"), &["cc", "-c", "/src/a.c"])], Status::Pass),
        ];

        for (entries, expected) in cases {
            let sut = check(&database(entries.clone()), &CountExpectation::default());

            assert_eq!(find(&sut, "non-empty-arguments").status, *expected, "entries: {entries:?}");
        }
    }

    #[test]
    fn non_empty_arguments_offender_is_just_the_file() {
        let sut = check(&database(vec![entry("/src/a.c", Some("a.o"), &[])]), &CountExpectation::default());

        let check = find(&sut, "non-empty-arguments");
        assert_eq!(check.offenders.len(), 1);
        assert_eq!(check.offenders[0].file, "/src/a.c");
        assert_eq!(check.offenders[0].output, None);
        assert_eq!(check.offenders[0].count, None);
    }

    #[test]
    fn non_empty_directory_fails_on_blank_and_passes_otherwise() {
        let blank = Entry { directory: String::new(), ..entry("/src/a.c", Some("a.o"), &["cc"]) };
        let cases: &[(Vec<Entry>, Status)] =
            &[(vec![blank], Status::Fail), (vec![entry("/src/a.c", Some("a.o"), &["cc"])], Status::Pass)];

        for (entries, expected) in cases {
            let sut = check(&database(entries.clone()), &CountExpectation::default());

            assert_eq!(find(&sut, "non-empty-directory").status, *expected, "entries: {entries:?}");
        }
    }

    #[test]
    fn byte_identical_entries_are_true_duplicates() {
        let one = entry("/src/a.c", Some("a.o"), &["cc", "-c", "/src/a.c", "-o", "a.o"]);

        let sut = check(&database(vec![one.clone(), one]), &CountExpectation::default());

        let check = find(&sut, "no-true-duplicates");
        assert_eq!(check.status, Status::Fail);
        assert_eq!(check.offenders.len(), 1);
        assert_eq!(check.offenders[0].count, Some(2));
    }

    #[test]
    fn multi_output_same_source_is_not_a_duplicate() {
        // The same source compiled into two objects with different flags forms two
        // distinct groups and must not be flagged.
        let shared =
            entry("/src/a.c", Some("shared.o"), &["cc", "-fPIC", "-c", "/src/a.c", "-o", "shared.o"]);
        let static_ = entry("/src/a.c", Some("static.o"), &["cc", "-c", "/src/a.c", "-o", "static.o"]);

        let sut = check(&database(vec![shared, static_]), &CountExpectation::default());

        assert_eq!(find(&sut, "no-true-duplicates").status, Status::Pass);
    }

    #[test]
    fn dependency_flag_only_difference_is_a_duplicate_after_dropping_flags() {
        // Two entries that differ only by a dependency flag are distinct as-is but
        // become true duplicates once `--drop-dependency-flags` normalizes them.
        let with_dep = entry("/src/a.c", Some("a.o"), &["cc", "-MD", "-c", "/src/a.c", "-o", "a.o"]);
        let without_dep = entry("/src/a.c", Some("a.o"), &["cc", "-c", "/src/a.c", "-o", "a.o"]);
        let cases: &[(bool, Status)] = &[(false, Status::Pass), (true, Status::Fail)];

        for (drop_flags, expected) in cases {
            let mut db = database(vec![with_dep.clone(), without_dep.clone()]);
            let norm = Normalization { drop_dependency_flags: *drop_flags, ..Default::default() };
            db.normalize(&norm);

            let sut = check(&db, &CountExpectation::default());

            assert_eq!(
                find(&sut, "no-true-duplicates").status,
                *expected,
                "drop_dependency_flags={drop_flags}"
            );
        }
    }

    #[test]
    fn entry_count_is_skipped_without_constraints() {
        let sut =
            check(&database(vec![entry("/src/a.c", Some("a.o"), &["cc"])]), &CountExpectation::default());

        let check = find(&sut, "entry-count");
        assert_eq!(check.status, Status::Skipped);
        assert!(check.detail.is_none());
    }

    #[test]
    fn entry_count_expected_objects_within_and_outside_tolerance() {
        // expected=10, tolerance=20% -> band ceil(10*20/100)=2, so [8, 12].
        let cases: &[(usize, Status)] =
            &[(8, Status::Pass), (12, Status::Pass), (7, Status::Fail), (13, Status::Fail)];

        for (count, expected) in cases {
            let entries: Vec<Entry> =
                (0..*count).map(|i| entry(&format!("/src/{i}.c"), None, &["cc"])).collect();
            let expectation =
                CountExpectation { expected_objects: Some(10), tolerance_pct: 20, ..Default::default() };

            let sut = check(&database(entries), &expectation);

            assert_eq!(find(&sut, "entry-count").status, *expected, "count={count}");
        }
    }

    #[test]
    fn entry_count_ceil_band_boundary_is_exercised() {
        // expected=3, tolerance=10% -> 3*10/100 = 0.3, ceil = 1, so band is [2, 4].
        // This pins the div_ceil rounding: a floor would give band 0 and reject 2/4.
        let expectation =
            CountExpectation { expected_objects: Some(3), tolerance_pct: 10, ..Default::default() };
        let cases: &[(usize, Status)] =
            &[(2, Status::Pass), (4, Status::Pass), (1, Status::Fail), (5, Status::Fail)];

        for (count, expected) in cases {
            let entries: Vec<Entry> =
                (0..*count).map(|i| entry(&format!("/src/{i}.c"), None, &["cc"])).collect();

            let sut = check(&database(entries), &expectation);

            assert_eq!(find(&sut, "entry-count").status, *expected, "count={count}");
        }
    }

    #[test]
    fn entry_count_min_entries_floor() {
        let cases: &[(usize, Status)] = &[(5, Status::Pass), (6, Status::Pass), (4, Status::Fail)];

        for (count, expected) in cases {
            let entries: Vec<Entry> =
                (0..*count).map(|i| entry(&format!("/src/{i}.c"), None, &["cc"])).collect();
            let expectation = CountExpectation { min_entries: Some(5), ..Default::default() };

            let sut = check(&database(entries), &expectation);

            assert_eq!(find(&sut, "entry-count").status, *expected, "count={count}");
        }
    }

    #[test]
    fn entry_count_both_constraints_must_hold() {
        // expected=10 tolerance=0 -> band exactly [10,10]; min=8. With 10 entries
        // both hold (pass); with 9 the expected band fails even though min holds.
        let cases: &[(usize, Status)] = &[(10, Status::Pass), (9, Status::Fail)];

        for (count, expected) in cases {
            let entries: Vec<Entry> =
                (0..*count).map(|i| entry(&format!("/src/{i}.c"), None, &["cc"])).collect();
            let expectation =
                CountExpectation { expected_objects: Some(10), tolerance_pct: 0, min_entries: Some(8) };

            let sut = check(&database(entries), &expectation);

            assert_eq!(find(&sut, "entry-count").status, *expected, "count={count}");
        }
    }

    #[test]
    fn report_pass_is_false_when_any_check_fails() {
        let sut = check(&database(vec![entry("/src/a.c", Some("a.o"), &[])]), &CountExpectation::default());

        assert!(!sut.pass);
    }
}
