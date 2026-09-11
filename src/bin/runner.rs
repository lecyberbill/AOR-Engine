// [WFGY] Zone: SAFE | λ: 0.25 | Fallbacks: 0/None | Action: Standalone game runner loading a packaged .aorpak without studio UI
#![allow(dead_code)]
use std::path::Path;
use rust_moteur_3d::studio::read_pak;

/// Point d'entrée du runner autonome : charge et valide une archive `.aorpak`.
///
/// Usage : `runner <chemin/vers/game.aorpak>`
fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: runner <game.aorpak>");
        std::process::exit(2);
    }

    let path = Path::new(&args[1]);
    match read_pak(path) {
        Ok(archive) => {
            let mut keys: Vec<&String> = archive.keys().collect();
            keys.sort();
            let total: usize = archive.values().map(|v| v.len()).sum();
            println!(
                "AOR Runner — '{}' chargé : {} fichiers, {} octets",
                path.display(),
                archive.len(),
                total
            );
            for key in keys {
                println!("  · {:<48} {:>8} o", key, archive[key].len());
            }
        }
        Err(err) => {
            eprintln!("Échec du chargement de l'archive: {}", err);
            std::process::exit(1);
        }
    }
}
