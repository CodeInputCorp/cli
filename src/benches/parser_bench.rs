use codeinput::core::parser::parse_line;
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use std::path::Path;

fn bench_parse_line_simple(c: &mut Criterion) {
    let source_path = Path::new("/test/CODEOWNERS");

    c.bench_function("parse_line_simple", |b| {
        b.iter(|| {
            parse_line(
                black_box("*.js @user"),
                black_box(1),
                black_box(source_path),
            )
        })
    });
}

criterion_group!(benches, bench_parse_line_simple);
criterion_main!(benches);
