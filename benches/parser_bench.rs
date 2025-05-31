use criterion::{black_box, criterion_group, criterion_main, Criterion};
use codeinput::core::parser::parse_codeowners;
use std::io::Write;
use tempfile::NamedTempFile;

fn benchmark_parser(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_codeowners_group");

    group.bench_function("parse_real_world_codeowners", |b| {
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let codeowners_content = br#"
# This is a comment
*.ts @owner1 @owner2
/docs/**/*.md @docs-owner @another-owner [docs]
# Another comment

/apps/ @app-owner1 @app-owner2 [infra] [app]
/libs/   @lib-owner # Inline comment
  /deep/nested/path/ @deep-owner1 @deep-owner2 @deep-owner3 [frontend]
# Empty lines follow


# Line with only spaces

# Line with tabs
\t\t
# Complex patterns
[mM]akefile @user1
src/**/*.java @java-dev @another-java-dev
*.{js,jsx,ts,tsx} @frontend-devs
/server/(app|test)/**/*.py @backend-devs [server]
docs/[^/]+/\.(md|txt)$ @doc-writers
"#;
        temp_file.write_all(codeowners_content).expect("Failed to write to temp file");
        let file_path = temp_file.path();

        b.iter(|| {
            parse_codeowners(black_box(file_path.to_str().unwrap()))
        })
    });

    group.finish();
}

criterion_group!(benches, benchmark_parser);
criterion_main!(benches);
