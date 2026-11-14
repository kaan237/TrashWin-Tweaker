use embed_manifest::manifest::ExecutionLevel;
use embed_manifest::{embed_manifest, new_manifest};

fn main() {
    // Registry HKLM ve servis ayarları yönetici izni ister; UAC'yi açılışta bir kez sor.
    if std::env::var_os("CARGO_CFG_WINDOWS").is_some() {
        embed_manifest(
            new_manifest("TrashWin").requested_execution_level(ExecutionLevel::RequireAdministrator),
        )
        .expect("manifest gömülemedi");
    }
    println!("cargo:rerun-if-changed=build.rs");
}
