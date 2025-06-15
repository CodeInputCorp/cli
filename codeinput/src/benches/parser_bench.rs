use codeinput::core::parser::{parse_codeowners, parse_line, parse_owner};
use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use std::io::Write;
use std::path::Path;
use tempfile::NamedTempFile;

// parse_codeowners benchmark
fn bench_parse_codeowners_small_file(c: &mut Criterion) {
    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "# Small CODEOWNERS file").unwrap();
    writeln!(temp_file, "*.js @frontend-team").unwrap();
    writeln!(temp_file, "*.rs @backend-team").unwrap();
    writeln!(temp_file, "/docs/ @docs-team #documentation").unwrap();
    writeln!(temp_file, "*.md @org/writers user@example.com #content").unwrap();
    temp_file.flush().unwrap();

    c.bench_function("parse_codeowners_small", |b| {
        b.iter(|| parse_codeowners(black_box(temp_file.path())).unwrap())
    });
}

fn bench_parse_codeowners_medium_file(c: &mut Criterion) {
    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "# Medium CODEOWNERS file with various patterns").unwrap();
    writeln!(temp_file).unwrap();
    writeln!(temp_file, "# Global owners").unwrap();
    writeln!(temp_file, "* @org/global-team").unwrap();
    writeln!(temp_file).unwrap();
    writeln!(temp_file, "# Frontend").unwrap();
    writeln!(temp_file, "*.js @frontend-team @alice #frontend").unwrap();
    writeln!(temp_file, "*.ts @frontend-team @bob #frontend #typescript").unwrap();
    writeln!(temp_file, "*.tsx @org/react-team user@react.com #ui #react").unwrap();
    writeln!(temp_file, "*.vue @org/vue-team #frontend #vue").unwrap();
    writeln!(temp_file, "/src/components/ @org/ui-team #components").unwrap();
    writeln!(temp_file, "/src/pages/ @org/frontend @charlie #pages").unwrap();
    writeln!(temp_file).unwrap();
    writeln!(temp_file, "# Backend").unwrap();
    writeln!(temp_file, "*.rs @backend-team @dave #backend #rust").unwrap();
    writeln!(
        temp_file,
        "*.go @org/go-team engineer@backend.com #backend #go"
    )
    .unwrap();
    writeln!(temp_file, "*.py @org/python-team #backend #python").unwrap();
    writeln!(temp_file, "/src/api/ @org/api-team #api").unwrap();
    writeln!(temp_file, "/src/database/ @org/db-team @eve #database").unwrap();
    writeln!(temp_file).unwrap();
    writeln!(temp_file, "# Infrastructure").unwrap();
    writeln!(temp_file, "Dockerfile @org/devops #docker #infrastructure").unwrap();
    writeln!(temp_file, "*.yml @org/devops @frank #ci #infrastructure").unwrap();
    writeln!(temp_file, "*.yaml @org/devops #ci #infrastructure").unwrap();
    writeln!(
        temp_file,
        "/terraform/ @org/infrastructure #terraform #infrastructure"
    )
    .unwrap();
    writeln!(temp_file, "/k8s/ @org/k8s-team ops@company.com #kubernetes").unwrap();
    writeln!(temp_file).unwrap();
    writeln!(temp_file, "# Documentation").unwrap();
    writeln!(temp_file, "*.md @docs-team @grace #documentation").unwrap();
    writeln!(temp_file, "/docs/ @org/tech-writers #documentation").unwrap();
    writeln!(temp_file, "README.md @org/maintainers #readme").unwrap();
    temp_file.flush().unwrap();

    c.bench_function("parse_codeowners_medium", |b| {
        b.iter(|| parse_codeowners(black_box(temp_file.path())).unwrap())
    });
}

fn bench_parse_codeowners_complex_file(c: &mut Criterion) {
    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(
        temp_file,
        "# Complex CODEOWNERS file with many patterns and edge cases"
    )
    .unwrap();
    writeln!(temp_file).unwrap();
    writeln!(temp_file, "# Global fallback").unwrap();
    writeln!(temp_file, "* @org/global-maintainers NOOWNER").unwrap();
    writeln!(temp_file).unwrap();
    writeln!(temp_file, "# Frontend microservices").unwrap();
    writeln!(temp_file, "/services/web-app/**/*.js @org/web-frontend @alice @bob user1@frontend.com #frontend #webapp").unwrap();
    writeln!(temp_file, "/services/web-app/**/*.ts @org/web-frontend @charlie engineer@webapp.com #frontend #typescript #webapp").unwrap();
    writeln!(temp_file, "/services/mobile-app/**/*.tsx @org/mobile-team @dave @eve mobile@company.com #mobile #react-native").unwrap();
    writeln!(
        temp_file,
        "/services/admin-portal/**/*.vue @org/admin-team admin@company.com #admin #vue #portal"
    )
    .unwrap();
    writeln!(temp_file).unwrap();
    writeln!(temp_file, "# Backend services").unwrap();
    writeln!(temp_file, "/services/user-service/**/*.rs @org/user-team @frank backend1@company.com #backend #rust #users").unwrap();
    writeln!(temp_file, "/services/payment-service/**/*.go @org/payments @grace @henry payments@company.com #backend #go #payments #critical").unwrap();
    writeln!(temp_file, "/services/notification-service/**/*.py @org/notifications notifications@company.com #backend #python #notifications").unwrap();
    writeln!(temp_file, "/services/analytics-service/**/*.scala @org/analytics @ivan analytics@company.com #backend #scala #analytics").unwrap();
    writeln!(temp_file).unwrap();
    writeln!(temp_file, "# Infrastructure and DevOps").unwrap();
    writeln!(temp_file, "/infrastructure/terraform/**/*.tf @org/infrastructure @jack @kate infra@company.com #terraform #infrastructure #critical").unwrap();
    writeln!(temp_file, "/infrastructure/k8s/**/*.yaml @org/k8s-team @liam k8s@company.com #kubernetes #infrastructure").unwrap();
    writeln!(temp_file, "/infrastructure/monitoring/**/*.yml @org/monitoring @mike monitoring@company.com #monitoring #infrastructure").unwrap();
    writeln!(
        temp_file,
        "/.github/workflows/**/*.yml @org/ci-cd @nina @oscar ci@company.com #ci #github-actions"
    )
    .unwrap();
    writeln!(temp_file).unwrap();
    writeln!(temp_file, "# Security and compliance").unwrap();
    writeln!(
        temp_file,
        "/security/**/* @org/security @paul @quinn security@company.com #security #critical"
    )
    .unwrap();
    writeln!(
        temp_file,
        "/**/*secret* @org/security security@company.com #security #secrets #critical"
    )
    .unwrap();
    writeln!(
        temp_file,
        "/**/*key* @org/security #security #secrets #critical"
    )
    .unwrap();
    writeln!(
        temp_file,
        "/compliance/**/*.json @org/compliance @rachel compliance@company.com #compliance #audit"
    )
    .unwrap();
    writeln!(temp_file).unwrap();
    writeln!(temp_file, "# Database and migrations").unwrap();
    writeln!(temp_file, "/database/migrations/**/*.sql @org/db-team @steve @tina db@company.com #database #migrations #critical").unwrap();
    writeln!(temp_file, "/database/schemas/**/*.sql @org/db-architects db-arch@company.com #database #schema #critical").unwrap();
    writeln!(temp_file).unwrap();
    writeln!(temp_file, "# Documentation and configuration").unwrap();
    writeln!(
        temp_file,
        "/docs/**/*.md @org/tech-writers @uma docs@company.com #documentation"
    )
    .unwrap();
    writeln!(
        temp_file,
        "/docs/api/**/*.md @org/api-docs @victor api-docs@company.com #documentation #api"
    )
    .unwrap();
    writeln!(
        temp_file,
        "/config/**/*.toml @org/config-team @walter config@company.com #configuration"
    )
    .unwrap();
    writeln!(
        temp_file,
        "/config/**/*.json @org/config-team config@company.com #configuration #json"
    )
    .unwrap();
    writeln!(temp_file).unwrap();
    writeln!(temp_file, "# Testing and quality").unwrap();
    writeln!(
        temp_file,
        "/tests/**/*.rs @org/qa-team @xander @yara qa@company.com #testing #rust"
    )
    .unwrap();
    writeln!(
        temp_file,
        "/tests/**/*.js @org/qa-frontend qa-frontend@company.com #testing #javascript"
    )
    .unwrap();
    writeln!(
        temp_file,
        "/benchmarks/**/* @org/performance @zoe perf@company.com #performance #benchmarks"
    )
    .unwrap();
    writeln!(temp_file).unwrap();
    writeln!(temp_file, "# Special files and patterns").unwrap();
    writeln!(
        temp_file,
        "Cargo.toml @org/rust-maintainers rust@company.com #rust #dependencies"
    )
    .unwrap();
    writeln!(
        temp_file,
        "package.json @org/js-maintainers js@company.com #javascript #dependencies"
    )
    .unwrap();
    writeln!(
        temp_file,
        "go.mod @org/go-maintainers go@company.com #go #dependencies"
    )
    .unwrap();
    writeln!(
        temp_file,
        "requirements.txt @org/python-maintainers python@company.com #python #dependencies"
    )
    .unwrap();
    writeln!(
        temp_file,
        "Dockerfile* @org/docker-team docker@company.com #docker #containers"
    )
    .unwrap();
    writeln!(
        temp_file,
        "*.dockerfile @org/docker-team #docker #containers"
    )
    .unwrap();
    writeln!(temp_file).unwrap();
    writeln!(temp_file, "# Root level important files").unwrap();
    writeln!(
        temp_file,
        "README.md @org/maintainers @alice @bob maintainers@company.com #readme #documentation"
    )
    .unwrap();
    writeln!(temp_file, "LICENSE @org/legal legal@company.com #legal").unwrap();
    writeln!(temp_file, "CODEOWNERS @org/maintainers #meta").unwrap();
    writeln!(temp_file, ".gitignore @org/maintainers #git #configuration").unwrap();
    temp_file.flush().unwrap();

    c.bench_function("parse_codeowners_complex", |b| {
        b.iter(|| parse_codeowners(black_box(temp_file.path())).unwrap())
    });
}

// parse_line benchmarks (5 variations)
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

fn bench_parse_line_multiple_owners(c: &mut Criterion) {
    let source_path = Path::new("/test/CODEOWNERS");

    c.bench_function("parse_line_multiple_owners", |b| {
        b.iter(|| {
            parse_line(
                black_box("*.ts @frontend @org/ui-team user@example.com"),
                black_box(1),
                black_box(source_path),
            )
        })
    });
}

fn bench_parse_line_with_tags(c: &mut Criterion) {
    let source_path = Path::new("/test/CODEOWNERS");

    c.bench_function("parse_line_with_tags", |b| {
        b.iter(|| {
            parse_line(
                black_box("/security/ @security-team #security #critical"),
                black_box(1),
                black_box(source_path),
            )
        })
    });
}

fn bench_parse_line_complex(c: &mut Criterion) {
    let source_path = Path::new("/test/CODEOWNERS");

    c.bench_function("parse_line_complex", |b| {
        b.iter(|| {
            parse_line(
                black_box("/src/components/**/*.tsx @org/frontend @alice @bob user@example.com #ui #react #frontend # Complex component ownership"),
                black_box(1),
                black_box(source_path),
            )
        })
    });
}

fn bench_parse_line_comment(c: &mut Criterion) {
    let source_path = Path::new("/test/CODEOWNERS");

    c.bench_function("parse_line_comment", |b| {
        b.iter(|| {
            parse_line(
                black_box("# This is just a comment line"),
                black_box(1),
                black_box(source_path),
            )
        })
    });
}

// parse_owner benchmarks (5 variations)
fn bench_parse_owner_user(c: &mut Criterion) {
    c.bench_function("parse_owner_user", |b| {
        b.iter(|| parse_owner(black_box("@username")).unwrap())
    });
}

fn bench_parse_owner_team(c: &mut Criterion) {
    c.bench_function("parse_owner_team", |b| {
        b.iter(|| parse_owner(black_box("@org/frontend-team")).unwrap())
    });
}

fn bench_parse_owner_email(c: &mut Criterion) {
    c.bench_function("parse_owner_email", |b| {
        b.iter(|| parse_owner(black_box("user.name+tag@subdomain.example.com")).unwrap())
    });
}

fn bench_parse_owner_unowned(c: &mut Criterion) {
    c.bench_function("parse_owner_unowned", |b| {
        b.iter(|| parse_owner(black_box("NOOWNER")).unwrap())
    });
}

fn bench_parse_owner_unknown(c: &mut Criterion) {
    c.bench_function("parse_owner_unknown", |b| {
        b.iter(|| parse_owner(black_box("some-random-text-123")).unwrap())
    });
}

criterion_group!(
    benches,
    bench_parse_codeowners_small_file,
    bench_parse_codeowners_medium_file,
    bench_parse_codeowners_complex_file,
    bench_parse_line_simple,
    bench_parse_line_multiple_owners,
    bench_parse_line_with_tags,
    bench_parse_line_complex,
    bench_parse_line_comment,
    bench_parse_owner_user,
    bench_parse_owner_team,
    bench_parse_owner_email,
    bench_parse_owner_unowned,
    bench_parse_owner_unknown
);
criterion_main!(benches);

