use codeinput::core::tag_resolver::{find_files_for_tag, find_tags_for_file};
use codeinput::core::types::{CodeownersEntry, FileEntry, Owner, OwnerType, Tag};
use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use std::path::{Path, PathBuf};

fn create_test_tag(name: &str) -> Tag {
    Tag(name.to_string())
}

fn create_test_owner(identifier: &str, owner_type: OwnerType) -> Owner {
    Owner {
        identifier: identifier.to_string(),
        owner_type,
    }
}

fn create_test_file_entry(path: &str, tags: Vec<Tag>) -> FileEntry {
    FileEntry {
        path: PathBuf::from(path),
        owners: vec![],
        tags,
    }
}

fn create_test_codeowners_entry(
    source_file: &str, line_number: usize, pattern: &str, tags: Vec<Tag>,
) -> CodeownersEntry {
    CodeownersEntry {
        source_file: PathBuf::from(source_file),
        line_number,
        pattern: pattern.to_string(),
        owners: vec![],
        tags,
    }
}

// find_files_for_tag benchmarks
fn bench_find_files_for_tag_small_dataset(c: &mut Criterion) {
    let target_tag = create_test_tag("frontend");
    let other_tag = create_test_tag("backend");

    let files = vec![
        create_test_file_entry("src/main.rs", vec![target_tag.clone()]),
        create_test_file_entry("src/lib.rs", vec![other_tag.clone()]),
        create_test_file_entry("src/utils.rs", vec![target_tag.clone()]),
        create_test_file_entry("tests/test.rs", vec![other_tag.clone()]),
        create_test_file_entry("docs/README.md", vec![target_tag.clone()]),
    ];

    c.bench_function("find_files_for_tag_small", |b| {
        b.iter(|| find_files_for_tag(black_box(&files), black_box(&target_tag)))
    });
}

fn bench_find_files_for_tag_medium_dataset(c: &mut Criterion) {
    let target_tag = create_test_tag("api");
    let mut files = Vec::new();

    // Create 100 files with mixed tags
    for i in 0..100 {
        let tags = if i % 3 == 0 {
            vec![target_tag.clone()]
        } else if i % 3 == 1 {
            vec![create_test_tag("backend")]
        } else {
            vec![create_test_tag("frontend")]
        };
        files.push(create_test_file_entry(&format!("src/file_{}.rs", i), tags));
    }

    c.bench_function("find_files_for_tag_medium", |b| {
        b.iter(|| find_files_for_tag(black_box(&files), black_box(&target_tag)))
    });
}

fn bench_find_files_for_tag_large_dataset(c: &mut Criterion) {
    let target_tag = create_test_tag("core");
    let mut files = Vec::new();

    // Create 1000 files with mixed tags
    for i in 0..1000 {
        let tags = if i % 10 == 0 {
            vec![target_tag.clone()]
        } else if i % 10 == 1 {
            vec![target_tag.clone(), create_test_tag("shared")]
        } else {
            vec![create_test_tag(&format!("module-{}", i % 5))]
        };
        files.push(create_test_file_entry(
            &format!("src/module_{}/file_{}.rs", i / 10, i),
            tags,
        ));
    }

    c.bench_function("find_files_for_tag_large", |b| {
        b.iter(|| find_files_for_tag(black_box(&files), black_box(&target_tag)))
    });
}

fn bench_find_files_for_tag_mega_large_dataset(c: &mut Criterion) {
    let target_tag = create_test_tag("core");
    let mut files = Vec::new();

    // Create 25,000 files with mixed tags
    for i in 0..25000 {
        let tags = if i % 100 == 0 {
            // 1% of files with target tag
            vec![target_tag.clone()]
        } else if i % 100 == 1 {
            // 1% of files with target + another tag
            vec![target_tag.clone(), create_test_tag("shared")]
        } else if i % 100 == 2 {
            // 1% of files with target + multiple tags
            vec![
                target_tag.clone(),
                create_test_tag(&format!("module-{}", i % 20)),
                create_test_tag(&format!("type-{}", i % 10)),
            ]
        } else {
            // 97% of files with other tags
            vec![create_test_tag(&format!("module-{}", i % 50))]
        };
        files.push(create_test_file_entry(
            &format!(
                "src/module_{}/submodule_{}/file_{}.rs",
                i / 1000,
                (i / 100) % 10,
                i
            ),
            tags,
        ));
    }

    c.bench_function("find_files_for_tag_mega_large", |b| {
        b.iter(|| find_files_for_tag(black_box(&files), black_box(&target_tag)))
    });
}

fn bench_find_files_for_tag_no_matches(c: &mut Criterion) {
    let target_tag = create_test_tag("nonexistent");
    let mut files = Vec::new();

    // Create 100 files with different tags
    for i in 0..100 {
        let tag = create_test_tag(&format!("tag-{}", i % 10));
        files.push(create_test_file_entry(
            &format!("src/file_{}.rs", i),
            vec![tag],
        ));
    }

    c.bench_function("find_files_for_tag_no_matches", |b| {
        b.iter(|| find_files_for_tag(black_box(&files), black_box(&target_tag)))
    });
}

fn bench_find_files_for_tag_multiple_tags_per_file(c: &mut Criterion) {
    let target_tag = create_test_tag("shared");
    let mut files = Vec::new();

    // Create 50 files, each with multiple tags
    for i in 0..50 {
        let tags = vec![
            target_tag.clone(),
            create_test_tag(&format!("module-{}", i % 5)),
            create_test_tag(&format!("type-{}", i % 3)),
        ];
        files.push(create_test_file_entry(&format!("src/file_{}.rs", i), tags));
    }

    c.bench_function("find_files_for_tag_multiple_tags", |b| {
        b.iter(|| find_files_for_tag(black_box(&files), black_box(&target_tag)))
    });
}

// find_tags_for_file benchmarks
fn bench_find_tags_for_file_simple_pattern(c: &mut Criterion) {
    let entries = vec![
        create_test_codeowners_entry(
            "/project/CODEOWNERS",
            1,
            "*.rs",
            vec![create_test_tag("rust")],
        ),
        create_test_codeowners_entry(
            "/project/CODEOWNERS",
            2,
            "*.js",
            vec![create_test_tag("javascript")],
        ),
    ];

    let file_path = Path::new("/project/src/main.rs");

    c.bench_function("find_tags_for_file_simple", |b| {
        b.iter(|| find_tags_for_file(black_box(file_path), black_box(&entries)).unwrap())
    });
}

fn bench_find_tags_for_file_complex_patterns(c: &mut Criterion) {
    let entries = vec![
        create_test_codeowners_entry(
            "/project/CODEOWNERS",
            1,
            "*",
            vec![create_test_tag("global")],
        ),
        create_test_codeowners_entry(
            "/project/CODEOWNERS",
            5,
            "src/**/*.rs",
            vec![create_test_tag("rust-source")],
        ),
        create_test_codeowners_entry(
            "/project/CODEOWNERS",
            10,
            "src/frontend/**/*",
            vec![create_test_tag("frontend")],
        ),
        create_test_codeowners_entry(
            "/project/CODEOWNERS",
            15,
            "src/frontend/**/*.tsx",
            vec![create_test_tag("react")],
        ),
        create_test_codeowners_entry(
            "/project/CODEOWNERS",
            20,
            "**/*test*",
            vec![create_test_tag("testing")],
        ),
    ];

    let file_path = Path::new("/project/src/frontend/components/Button.tsx");

    c.bench_function("find_tags_for_file_complex", |b| {
        b.iter(|| find_tags_for_file(black_box(file_path), black_box(&entries)).unwrap())
    });
}

fn bench_find_tags_for_file_many_entries(c: &mut Criterion) {
    let mut entries = Vec::new();

    // Create many entries with different patterns
    for i in 0..100 {
        entries.push(create_test_codeowners_entry(
            "/project/CODEOWNERS",
            i + 1,
            &format!("src/module_{}/**/*", i),
            vec![create_test_tag(&format!("module-{}", i))],
        ));
    }

    // Add some general patterns
    entries.push(create_test_codeowners_entry(
        "/project/CODEOWNERS",
        101,
        "*.rs",
        vec![create_test_tag("rust")],
    ));

    let file_path = Path::new("/project/src/main.rs");

    c.bench_function("find_tags_for_file_many_entries", |b| {
        b.iter(|| find_tags_for_file(black_box(file_path), black_box(&entries)).unwrap())
    });
}

fn bench_find_tags_for_file_nested_codeowners(c: &mut Criterion) {
    let entries = vec![
        // Root CODEOWNERS
        create_test_codeowners_entry("/project/CODEOWNERS", 1, "*", vec![create_test_tag("root")]),
        create_test_codeowners_entry(
            "/project/CODEOWNERS",
            5,
            "src/**/*",
            vec![create_test_tag("source")],
        ),
        // Nested CODEOWNERS in src/
        create_test_codeowners_entry(
            "/project/src/CODEOWNERS",
            1,
            "*.rs",
            vec![create_test_tag("rust")],
        ),
        create_test_codeowners_entry(
            "/project/src/CODEOWNERS",
            3,
            "frontend/**/*",
            vec![create_test_tag("frontend")],
        ),
        // Nested CODEOWNERS in src/frontend/
        create_test_codeowners_entry(
            "/project/src/frontend/CODEOWNERS",
            1,
            "*.tsx",
            vec![create_test_tag("react")],
        ),
        create_test_codeowners_entry(
            "/project/src/frontend/CODEOWNERS",
            2,
            "components/**/*",
            vec![create_test_tag("components")],
        ),
    ];

    let file_path = Path::new("/project/src/frontend/components/Button.tsx");

    c.bench_function("find_tags_for_file_nested", |b| {
        b.iter(|| find_tags_for_file(black_box(file_path), black_box(&entries)).unwrap())
    });
}

fn bench_find_tags_for_file_no_matches(c: &mut Criterion) {
    let entries = vec![
        create_test_codeowners_entry(
            "/other/CODEOWNERS",
            1,
            "*.py",
            vec![create_test_tag("python")],
        ),
        create_test_codeowners_entry(
            "/different/CODEOWNERS",
            1,
            "*.go",
            vec![create_test_tag("golang")],
        ),
    ];

    let file_path = Path::new("/project/src/main.rs");

    c.bench_function("find_tags_for_file_no_matches", |b| {
        b.iter(|| find_tags_for_file(black_box(file_path), black_box(&entries)).unwrap())
    });
}

fn bench_find_tags_for_file_priority_resolution(c: &mut Criterion) {
    let entries = vec![
        create_test_codeowners_entry(
            "/project/CODEOWNERS",
            1,
            "*",
            vec![create_test_tag("global")],
        ),
        create_test_codeowners_entry(
            "/project/CODEOWNERS",
            5,
            "*.rs",
            vec![create_test_tag("rust")],
        ),
        create_test_codeowners_entry(
            "/project/CODEOWNERS",
            10,
            "src/*.rs",
            vec![create_test_tag("src-rust")],
        ),
        create_test_codeowners_entry(
            "/project/CODEOWNERS",
            15,
            "src/main.rs",
            vec![create_test_tag("main")],
        ),
        create_test_codeowners_entry(
            "/project/CODEOWNERS",
            20,
            "**/*.rs",
            vec![create_test_tag("all-rust")],
        ),
    ];

    let file_path = Path::new("/project/src/main.rs");

    c.bench_function("find_tags_for_file_priority", |b| {
        b.iter(|| find_tags_for_file(black_box(file_path), black_box(&entries)).unwrap())
    });
}

fn bench_find_tags_for_file_multiple_tags_per_entry(c: &mut Criterion) {
    let entries = vec![
        create_test_codeowners_entry(
            "/project/CODEOWNERS",
            1,
            "*.rs",
            vec![
                create_test_tag("rust"),
                create_test_tag("backend"),
                create_test_tag("core"),
            ],
        ),
        create_test_codeowners_entry(
            "/project/CODEOWNERS",
            5,
            "src/api/**/*",
            vec![
                create_test_tag("api"),
                create_test_tag("service"),
                create_test_tag("public"),
            ],
        ),
    ];

    let file_path = Path::new("/project/src/api/users.rs");

    c.bench_function("find_tags_for_file_multiple_tags_per_entry", |b| {
        b.iter(|| find_tags_for_file(black_box(file_path), black_box(&entries)).unwrap())
    });
}

fn bench_find_tags_for_file_deep_hierarchy(c: &mut Criterion) {
    let mut entries = Vec::new();

    // Create entries for different levels of hierarchy
    let hierarchy_levels = vec![
        ("*", "root"),
        ("src/**/*", "source"),
        ("src/backend/**/*", "backend"),
        ("src/backend/api/**/*", "api"),
        ("src/backend/api/v1/**/*", "v1"),
        ("src/backend/api/v1/users/**/*", "users"),
    ];

    for (i, (pattern, tag_name)) in hierarchy_levels.iter().enumerate() {
        entries.push(create_test_codeowners_entry(
            "/project/CODEOWNERS",
            i + 1,
            pattern,
            vec![create_test_tag(tag_name)],
        ));
    }

    let file_path = Path::new("/project/src/backend/api/v1/users/controller.rs");

    c.bench_function("find_tags_for_file_deep_hierarchy", |b| {
        b.iter(|| find_tags_for_file(black_box(file_path), black_box(&entries)).unwrap())
    });
}

criterion_group!(
    benches,
    bench_find_files_for_tag_small_dataset,
    bench_find_files_for_tag_medium_dataset,
    bench_find_files_for_tag_large_dataset,
    bench_find_files_for_tag_mega_large_dataset,
    bench_find_files_for_tag_no_matches,
    bench_find_files_for_tag_multiple_tags_per_file,
    bench_find_tags_for_file_simple_pattern,
    bench_find_tags_for_file_complex_patterns,
    bench_find_tags_for_file_many_entries,
    bench_find_tags_for_file_nested_codeowners,
    bench_find_tags_for_file_no_matches,
    bench_find_tags_for_file_priority_resolution,
    bench_find_tags_for_file_multiple_tags_per_entry,
    bench_find_tags_for_file_deep_hierarchy
);
criterion_main!(benches);
