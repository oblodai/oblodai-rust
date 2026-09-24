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

/// `names.2.0.txt` is the method list 2.0.0 shipped with, written once and never regenerated:
/// the migration guide maps 1.x to exactly these, and a method a later contract adds does not
/// need a 1.x row.
#[test]
fn the_2_0_migration_maps_every_2_0_name() {
    let text = read("MIGRATION-2.0.md");
    let missing: Vec<String> = read("names.2.0.txt")
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

/// `resource.method` of every generated method, from the accessors and impls of resources.rs.
fn generated_methods() -> Vec<String> {
    let resources = read("src/generated/resources.rs");
    let mut accessors = std::collections::HashMap::new();
    for line in resources.lines() {
        // `pub fn payments(&self) -> $crate::generated::resources::Payments<$tr> {`
        if let Some(rest) = line.trim().strip_prefix("pub fn ") {
            if let Some((name, tail)) = rest.split_once("(&self) -> $crate::generated::resources::")
            {
                accessors.insert(
                    tail.split('<').next().unwrap().to_string(),
                    name.to_string(),
                );
            }
        }
    }
    let mut out = Vec::new();
    let mut current = None;
    for line in resources.lines() {
        if let Some(rest) = line.strip_prefix("impl<Tr: Clone> ") {
            current = accessors.get(rest.split('<').next().unwrap()).cloned();
        } else if let (Some(res), Some(rest)) = (&current, line.strip_prefix("    pub fn ")) {
            let method = rest.split(['(', '<']).next().unwrap();
            if method != "new" {
                out.push(format!("{res}.{method}"));
            }
        }
    }
    out.sort();
    out
}

#[test]
fn names_lock_is_every_generated_method() {
    let mut locked: Vec<String> = read("names.lock")
        .split_whitespace()
        .map(str::to_string)
        .collect();
    locked.sort();
    let generated = generated_methods();
    assert!(generated.len() > 100, "parsed {} methods", generated.len());
    assert_eq!(locked, generated, "names.lock and src/generated disagree");
}

/// The method table between the sdkgen markers is the generator's: every locked name is in it.
#[test]
fn the_readme_method_table_lists_every_locked_name() {
    for readme in ["README.md", "README.ru.md"] {
        let text = read(readme);
        let table = text
            .split_once("<!-- sdkgen:methods -->")
            .and_then(|(_, rest)| rest.split_once("<!-- /sdkgen:methods -->"))
            .unwrap_or_else(|| panic!("{readme} has no sdkgen:methods section"))
            .0;
        for name in read("names.lock").split_whitespace() {
            let (resource, method) = name.split_once('.').unwrap();
            let row = table
                .lines()
                .find(|l| l.starts_with(&format!("| `{resource}()` |")))
                .unwrap_or_else(|| panic!("{readme}: no row for {resource}()"));
            assert!(
                row.contains(&format!("`{method}`")),
                "{readme}: {name} missing from the method table"
            );
        }
    }
}

#[test]
fn version_and_msrv_agree_everywhere() {
    let version = cargo_field("version");
    assert_eq!(version, env!("CARGO_PKG_VERSION"));
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
