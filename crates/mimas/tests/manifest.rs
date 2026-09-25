use std::{
    fs::File,
    path::PathBuf,
    time::{Duration, SystemTime},
};

use api::{ApiConstant, ApiEntry, Library, Manifest, ManifestRef};
use mimas::{Literal, Ty, Vm, mimas, write_api};
use vm::export::{manifest_path, target_dir};

#[mimas]
fn cheer(name: String) -> String {
    format!("go {name}!")
}

#[mimas]
struct Hero {
    name: String,
    health: i64,
}

#[mimas]
impl Hero {
    fn hurt(&mut self, amount: i64) {
        self.health -= amount;
    }
}

#[mimas]
enum Mood {
    Calm,
    Angry,
}

#[mimas(game)]
const MAX: i64 = 100;

const HOST_SCRIPT: &str = r#"
let hero = Hero { name = cheer("ferris"), health = game::MAX };
hero.hurt(10);
let mood = match Mood::Angry {
    Mood::Calm => hero.health,
    Mood::Angry => 0,
};
"#;

const STD_SCRIPT: &str = r#"
use std::math::Vec3;

fn half(n: int) -> int! {
    if n % 2 != 0 {
        raise "odd";
    }
    n ~/ 2
}

let xs = [1, 2, 3];
xs.push(4);
let d = ~{ a = 1 };
let a: int? = d["a"];
let total = xs.sum() + (a ?? 0);
let shout = f"{total}".to_upper();
let length = Vec3 { x = 2.0, y = 3.0, z = 6.0 }.length();
let halved = half(total) absolve |_| 0;
let here = std::fs::cwd();
"#;

fn installed() -> Library<()> {
    Vm::new().install_library(mimas::library::std)
}

fn round_trip(library: &Library<()>) -> Library<()> {
    let json = serde_json::to_string(&ManifestRef {
        version: api::VERSION,
        library,
    })
    .unwrap();
    serde_json::from_str::<Manifest>(&json).unwrap().library
}

fn errors(source: &str, library: &Library<()>) -> Vec<String> {
    solve::Modules::from_files([("main.mim", source)], library)
        .errors
        .iter()
        .map(|e| format!("{e:?}"))
        .collect()
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("mimas-manifest-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn manifest_round_trip() {
    let library = installed();
    let json = serde_json::to_string(&ManifestRef {
        version: api::VERSION,
        library: &library,
    })
    .unwrap();
    let manifest: Manifest = serde_json::from_str(&json).unwrap();
    assert_eq!(manifest.version, api::VERSION);
    let again = serde_json::to_string(&ManifestRef {
        version: &manifest.version,
        library: &manifest.library,
    })
    .unwrap();
    assert_eq!(json, again);
}

#[test]
fn library_round_trip() {
    let original = installed();
    let loaded = round_trip(&original);

    let script = format!("{STD_SCRIPT}\n{HOST_SCRIPT}");
    assert_eq!(errors(&script, &original), Vec::<String>::new());
    assert_eq!(errors(&script, &loaded), Vec::<String>::new());

    let broken = "let xs = [1, 2, 3];\nlet n: str = xs.len();";
    let expected = errors(broken, &original);
    assert_eq!(expected.len(), 1);
    assert_eq!(errors(broken, &loaded), expected);
}

#[test]
fn resolve_host_items() {
    assert!(!errors(HOST_SCRIPT, &Library::new()).is_empty());
    assert_eq!(
        errors(HOST_SCRIPT, &round_trip(&installed())),
        Vec::<String>::new()
    );
}

#[test]
fn reject_non_finite_constants() {
    let mut library = Library::new();
    library.constant(ApiConstant {
        name: "BROKEN".into(),
        module: Vec::new(),
        recv_ty: None,
        ty: Ty::Float,
        value: Literal::Float(f64::NAN),
        doc: String::new(),
    });
    let dir = scratch("non-finite");
    let path = dir.join("api.json");

    let error = write_api(&library, &path).unwrap_err();
    assert!(error.to_string().contains("BROKEN"), "{error}");
    assert!(!path.exists());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn unchanged_manifest_is_not_rewritten() {
    let library = installed();
    let dir = scratch("unchanged");
    let path = dir.join("mimas").join("api.json");

    write_api(&library, &path).unwrap();
    let old = SystemTime::now() - Duration::from_secs(60 * 60);
    File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_modified(old)
        .unwrap();
    write_api(&library, &path).unwrap();
    assert_eq!(std::fs::metadata(&path).unwrap().modified().unwrap(), old);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn target_dir_finds_the_cachedir_tag() {
    let dir = scratch("cachedir");
    std::fs::write(dir.join("CACHEDIR.TAG"), "").unwrap();
    assert_eq!(
        target_dir(&dir.join("debug").join("app")),
        Some(dir.as_path())
    );
    assert_eq!(
        target_dir(&dir.join("release").join("examples").join("ex")),
        Some(dir.as_path())
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn target_dir_skips_test_binaries() {
    let dir = scratch("deps");
    std::fs::write(dir.join("CACHEDIR.TAG"), "").unwrap();
    assert_eq!(
        target_dir(&dir.join("debug").join("deps").join("app-1234")),
        None
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn target_dir_needs_a_cargo_target() {
    let dir = scratch("no-target");
    assert_eq!(target_dir(&dir.join("debug").join("app")), None);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn manifest_is_named_after_the_binary() {
    let dir = scratch("named");
    std::fs::write(dir.join("CACHEDIR.TAG"), "").unwrap();
    assert_eq!(
        manifest_path(&dir.join("debug").join("game-loop")),
        Some(dir.join("mimas").join("game-loop.json"))
    );
    assert_eq!(
        manifest_path(&dir.join("release").join("examples").join("ex")),
        Some(dir.join("mimas").join("ex.json"))
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn host_constants_link_to_their_entry() {
    let library = round_trip(&installed());
    let loaded = solve::Modules::from_files([("main.mim", HOST_SCRIPT)], &library);
    assert!(loaded.errors.is_empty());
    let resolutions = solve::Resolutions::from(loaded.solver);
    let (dec, _) = resolutions
        .decs
        .iter()
        .find(|(_, dec)| dec.name == "MAX")
        .unwrap();
    let native = resolutions.native_constants[&dec];
    let (_, entry) = library.natives().find(|(id, _)| *id == native).unwrap();
    assert!(matches!(entry, ApiEntry::Constant(c) if c.name == "MAX"));
}
