//! The documents tell the truth about this version: every locked name is in the migration table,
//! the version and the MSRV agree everywhere, nothing points at what 2.0 removed.

fn read(name: &str) -> String {
    std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(name))
        .unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn cargo_field(name: &str) -> String {
    let toml = read("Cargo.toml");
    let line = toml
        .lines()
        .find(|l| l.starts_with(&format!("{name} = ")))
        .unwrap_or_else(|| panic!("no {name} in Cargo.toml"));
    line.split('"').nth(1).unwrap().to_string()
}

#[test]
fn the_2_0_migration_maps_every_locked_name() {
    let text = read("MIGRATION-2.0.md");
    let missing: Vec<String> = read("names.lock")
        .split_whitespace()
        .map(|n| {
            let (resource, method) = n.split_once('.').unwrap();
            format!("| `{resource}().{method}()` |")
        })
        .filter(|row| !text.contains(row.as_str()))
        .collect();
    assert!(
        missing.is_empty(),
        "MIGRATION-2.0.md does not map {missing:?}"
    );
}

#[test]
fn every_locked_name_is_a_generated_method() {
    let resources = read("src/generated/resources.rs");
    for name in read("names.lock").split_whitespace() {
        let method = name.split_once('.').unwrap().1;
        assert!(
            resources.contains(&format!("    pub fn {method}(")),
            "names.lock has {name}, src/generated has no such method"
        );
    }
}

#[test]
fn version_and_msrv_agree_everywhere() {
    let version = cargo_field("version");
    assert_eq!(version, "2.0.0");
    assert!(read("CHANGELOG.md").contains(&format!("## [{version}]")));
    let major_minor = version.rsplit_once('.').unwrap().0.to_string();
    for readme in ["README.md", "README.ru.md"] {
        assert!(
            read(readme).contains(&format!("oblodai = \"{major_minor}\"")),
            "{readme} installs another version"
        );
    }
    let msrv = cargo_field("rust-version");
    assert_eq!(msrv, "1.86");
    for doc in ["README.md", "README.ru.md", "AGENTS.md", "src/lib.rs"] {
        assert!(
            read(doc).contains(&msrv),
            "{doc} does not state the MSRV {msrv}"
        );
    }
    assert_eq!(oblodai::SDK_VERSION, version);
}

#[test]
fn nothing_points_at_what_2_0_removed() {
    for doc in [
        "README.md",
        "README.ru.md",
        "AGENTS.md",
        "RELEASING.md",
        "examples/README.md",
    ] {
        let text = read(doc);
        for gone in [
            "contract::",
            "codegen.py",
            "merchants()",
            "RequestBuilder",
            "FileBuilder",
            "PageParams",
            ".send_raw()",
            "ERROR_CODES",
        ] {
            assert!(!text.contains(gone), "{doc} still mentions {gone}");
        }
    }
}
