use codeinput::core::owner_resolver::{find_files_for_owner, find_owners_for_file};
use codeinput::core::types::{CodeownersEntry, FileEntry, Owner, OwnerType, Tag};
use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use std::path::{Path, PathBuf};

fn create_test_owner(identifier: &str, owner_type: OwnerType) -> Owner {
    Owner {
        identifier: identifier.to_string(),
        owner_type,
    }
}

fn create_test_file_entry(path: &str, owners: Vec<Owner>) -> FileEntry {
    FileEntry {
        path: PathBuf::from(path),
        owners,
        tags: vec![],
    }
}

fn create_test_codeowners_entry(
    source_file: &str, line_number: usize, pattern: &str, owners: Vec<Owner>,
) -> CodeownersEntry {
    CodeownersEntry {
        source_file: PathBuf::from(source_file),
        line_number,
        pattern: pattern.to_string(),
        owners,
        tags: vec![],
    }
}

// find_files_for_owner benchmarks
fn bench_find_files_for_owner_small_dataset(c: &mut Criterion) {
    let target_owner = create_test_owner("@frontend-team", OwnerType::Team);
    let other_owner = create_test_owner("@backend-team", OwnerType::Team);

    let files = vec![
        create_test_file_entry("src/main.rs", vec![target_owner.clone()]),
        create_test_file_entry("src/lib.rs", vec![other_owner.clone()]),
        create_test_file_entry("src/utils.rs", vec![target_owner.clone()]),
        create_test_file_entry("tests/test.rs", vec![other_owner.clone()]),
        create_test_file_entry("docs/README.md", vec![target_owner.clone()]),
    ];

    c.bench_function("find_files_for_owner_small", |b| {
        b.iter(|| find_files_for_owner(black_box(&files), black_box(&target_owner)))
    });
}

fn bench_find_files_for_owner_medium_dataset(c: &mut Criterion) {
    let target_owner = create_test_owner("@frontend-team", OwnerType::Team);
    let mut files = Vec::new();

    // Create 100 files with mixed ownership
    for i in 0..100 {
        let owner = if i % 3 == 0 {
            target_owner.clone()
        } else if i % 3 == 1 {
            create_test_owner("@backend-team", OwnerType::Team)
        } else {
            create_test_owner("@devops-team", OwnerType::Team)
        };
        files.push(create_test_file_entry(
            &format!("src/file_{}.rs", i),
            vec![owner],
        ));
    }

    c.bench_function("find_files_for_owner_medium", |b| {
        b.iter(|| find_files_for_owner(black_box(&files), black_box(&target_owner)))
    });
}

fn bench_find_files_for_owner_large_dataset(c: &mut Criterion) {
    let target_owner = create_test_owner("@frontend-team", OwnerType::Team);
    let mut files = Vec::new();

    // Create 1000 files with mixed ownership
    for i in 0..1000 {
        let owners = if i % 10 == 0 {
            vec![target_owner.clone()]
        } else if i % 10 == 1 {
            vec![
                target_owner.clone(),
                create_test_owner("@backend-team", OwnerType::Team),
            ]
        } else {
            vec![create_test_owner(
                &format!("@team-{}", i % 5),
                OwnerType::Team,
            )]
        };
        files.push(create_test_file_entry(
            &format!("src/module_{}/file_{}.rs", i / 10, i),
            owners,
        ));
    }

    c.bench_function("find_files_for_owner_large", |b| {
        b.iter(|| find_files_for_owner(black_box(&files), black_box(&target_owner)))
    });
}

fn bench_find_files_for_owner_mega_large_dataset(c: &mut Criterion) {
    let target_owner = create_test_owner("@frontend-team", OwnerType::Team);
    let mut files = Vec::new();

    // Create 25,000 files with mixed ownership
    for i in 0..25000 {
        let owners = if i % 100 == 0 {
            // 1% of files owned by target
            vec![target_owner.clone()]
        } else if i % 100 == 1 {
            // 1% of files with target + another owner
            vec![
                target_owner.clone(),
                create_test_owner("@backend-team", OwnerType::Team),
            ]
        } else if i % 100 == 2 {
            // 1% of files with target + multiple others
            vec![
                target_owner.clone(),
                create_test_owner(&format!("@team-{}", i % 20), OwnerType::Team),
                create_test_owner(&format!("user-{}", i % 10), OwnerType::User),
            ]
        } else {
            // 97% of files with other owners
            vec![create_test_owner(
                &format!("@team-{}", i % 50),
                OwnerType::Team,
            )]
        };
        files.push(create_test_file_entry(
            &format!(
                "src/module_{}/submodule_{}/file_{}.rs",
                i / 1000,
                (i / 100) % 10,
                i
            ),
            owners,
        ));
    }

    c.bench_function("find_files_for_owner_mega_large", |b| {
        b.iter(|| find_files_for_owner(black_box(&files), black_box(&target_owner)))
    });
}

fn bench_find_files_for_owner_no_matches(c: &mut Criterion) {
    let target_owner = create_test_owner("@nonexistent-team", OwnerType::Team);
    let mut files = Vec::new();

    // Create 100 files with different owners
    for i in 0..100 {
        let owner = create_test_owner(&format!("@team-{}", i % 10), OwnerType::Team);
        files.push(create_test_file_entry(
            &format!("src/file_{}.rs", i),
            vec![owner],
        ));
    }

    c.bench_function("find_files_for_owner_no_matches", |b| {
        b.iter(|| find_files_for_owner(black_box(&files), black_box(&target_owner)))
    });
}

fn bench_find_files_for_owner_multiple_owners_per_file(c: &mut Criterion) {
    let target_owner = create_test_owner("@frontend-team", OwnerType::Team);
    let mut files = Vec::new();

    // Create 50 files, each with multiple owners
    for i in 0..50 {
        let owners = vec![
            target_owner.clone(),
            create_test_owner(&format!("@team-{}", i % 5), OwnerType::Team),
            create_test_owner(&format!("user-{}", i % 3), OwnerType::User),
        ];
        files.push(create_test_file_entry(
            &format!("src/file_{}.rs", i),
            owners,
        ));
    }

    c.bench_function("find_files_for_owner_multiple_owners", |b| {
        b.iter(|| find_files_for_owner(black_box(&files), black_box(&target_owner)))
    });
}

// find_owners_for_file benchmarks
fn bench_find_owners_for_file_simple_pattern(c: &mut Criterion) {
    let entries = vec![
        create_test_codeowners_entry(
            "/project/CODEOWNERS",
            1,
            "*.rs",
            vec![create_test_owner("@rust-team", OwnerType::Team)],
        ),
        create_test_codeowners_entry(
            "/project/CODEOWNERS",
            2,
            "*.js",
            vec![create_test_owner("@frontend-team", OwnerType::Team)],
        ),
    ];

    let file_path = Path::new("/project/src/main.rs");

    c.bench_function("find_owners_for_file_simple", |b| {
        b.iter(|| find_owners_for_file(black_box(file_path), black_box(&entries)).unwrap())
    });
}

fn bench_find_owners_for_file_complex_patterns(c: &mut Criterion) {
    let entries = vec![
        create_test_codeowners_entry(
            "/project/CODEOWNERS",
            1,
            "*",
            vec![create_test_owner("@global-team", OwnerType::Team)],
        ),
        create_test_codeowners_entry(
            "/project/CODEOWNERS",
            5,
            "src/**/*.rs",
            vec![create_test_owner("@rust-team", OwnerType::Team)],
        ),
        create_test_codeowners_entry(
            "/project/CODEOWNERS",
            10,
            "src/frontend/**/*",
            vec![create_test_owner("@frontend-team", OwnerType::Team)],
        ),
        create_test_codeowners_entry(
            "/project/CODEOWNERS",
            15,
            "src/frontend/**/*.tsx",
            vec![create_test_owner("@react-team", OwnerType::Team)],
        ),
        create_test_codeowners_entry(
            "/project/CODEOWNERS",
            20,
            "**/*test*",
            vec![create_test_owner("@qa-team", OwnerType::Team)],
        ),
    ];

    let file_path = Path::new("/project/src/frontend/components/Button.tsx");

    c.bench_function("find_owners_for_file_complex", |b| {
        b.iter(|| find_owners_for_file(black_box(file_path), black_box(&entries)).unwrap())
    });
}

fn bench_find_owners_for_file_many_entries(c: &mut Criterion) {
    let mut entries = Vec::new();

    // Create many entries with different patterns
    for i in 0..100 {
        entries.push(create_test_codeowners_entry(
            "/project/CODEOWNERS",
            i + 1,
            &format!("src/module_{}/**/*", i),
            vec![create_test_owner(&format!("@team-{}", i), OwnerType::Team)],
        ));
    }

    // Add some general patterns
    entries.push(create_test_codeowners_entry(
        "/project/CODEOWNERS",
        101,
        "*.rs",
        vec![create_test_owner("@rust-team", OwnerType::Team)],
    ));

    let file_path = Path::new("/project/src/main.rs");

    c.bench_function("find_owners_for_file_many_entries", |b| {
        b.iter(|| find_owners_for_file(black_box(file_path), black_box(&entries)).unwrap())
    });
}

fn bench_find_owners_for_file_nested_codeowners(c: &mut Criterion) {
    let entries = vec![
        // Root CODEOWNERS
        create_test_codeowners_entry(
            "/project/CODEOWNERS",
            1,
            "*",
            vec![create_test_owner("@root-team", OwnerType::Team)],
        ),
        create_test_codeowners_entry(
            "/project/CODEOWNERS",
            5,
            "src/**/*",
            vec![create_test_owner("@src-team", OwnerType::Team)],
        ),
        // Nested CODEOWNERS in src/
        create_test_codeowners_entry(
            "/project/src/CODEOWNERS",
            1,
            "*.rs",
            vec![create_test_owner("@rust-team", OwnerType::Team)],
        ),
        create_test_codeowners_entry(
            "/project/src/CODEOWNERS",
            3,
            "frontend/**/*",
            vec![create_test_owner("@frontend-team", OwnerType::Team)],
        ),
        // Nested CODEOWNERS in src/frontend/
        create_test_codeowners_entry(
            "/project/src/frontend/CODEOWNERS",
            1,
            "*.tsx",
            vec![create_test_owner("@react-team", OwnerType::Team)],
        ),
        create_test_codeowners_entry(
            "/project/src/frontend/CODEOWNERS",
            2,
            "components/**/*",
            vec![create_test_owner("@ui-team", OwnerType::Team)],
        ),
    ];

    let file_path = Path::new("/project/src/frontend/components/Button.tsx");

    c.bench_function("find_owners_for_file_nested", |b| {
        b.iter(|| find_owners_for_file(black_box(file_path), black_box(&entries)).unwrap())
    });
}

fn bench_find_owners_for_file_no_matches(c: &mut Criterion) {
    let entries = vec![
        create_test_codeowners_entry(
            "/other/CODEOWNERS",
            1,
            "*.py",
            vec![create_test_owner("@python-team", OwnerType::Team)],
        ),
        create_test_codeowners_entry(
            "/different/CODEOWNERS",
            1,
            "*.go",
            vec![create_test_owner("@go-team", OwnerType::Team)],
        ),
    ];

    let file_path = Path::new("/project/src/main.rs");

    c.bench_function("find_owners_for_file_no_matches", |b| {
        b.iter(|| find_owners_for_file(black_box(file_path), black_box(&entries)).unwrap())
    });
}

fn bench_find_owners_for_file_priority_resolution(c: &mut Criterion) {
    let entries = vec![
        create_test_codeowners_entry(
            "/project/CODEOWNERS",
            1,
            "*",
            vec![create_test_owner("@global-team", OwnerType::Team)],
        ),
        create_test_codeowners_entry(
            "/project/CODEOWNERS",
            5,
            "*.rs",
            vec![create_test_owner("@rust-team", OwnerType::Team)],
        ),
        create_test_codeowners_entry(
            "/project/CODEOWNERS",
            10,
            "src/*.rs",
            vec![create_test_owner("@src-rust-team", OwnerType::Team)],
        ),
        create_test_codeowners_entry(
            "/project/CODEOWNERS",
            15,
            "src/main.rs",
            vec![create_test_owner("@main-team", OwnerType::Team)],
        ),
        create_test_codeowners_entry(
            "/project/CODEOWNERS",
            20,
            "**/*.rs",
            vec![create_test_owner("@all-rust-team", OwnerType::Team)],
        ),
    ];

    let file_path = Path::new("/project/src/main.rs");

    c.bench_function("find_owners_for_file_priority", |b| {
        b.iter(|| find_owners_for_file(black_box(file_path), black_box(&entries)).unwrap())
    });
}

criterion_group!(
    benches,
    bench_find_files_for_owner_small_dataset,
    bench_find_files_for_owner_medium_dataset,
    bench_find_files_for_owner_large_dataset,
    bench_find_files_for_owner_mega_large_dataset,
    bench_find_files_for_owner_no_matches,
    bench_find_files_for_owner_multiple_owners_per_file,
    bench_find_owners_for_file_simple_pattern,
    bench_find_owners_for_file_complex_patterns,
    bench_find_owners_for_file_many_entries,
    bench_find_owners_for_file_nested_codeowners,
    bench_find_owners_for_file_no_matches,
    bench_find_owners_for_file_priority_resolution
);
criterion_main!(benches);
