use std::{env, fs, path::PathBuf};

use ferric_theme::textures::{
    render_card, render_grain, render_plinth, write_best_rgb_png, write_best_rgba_png,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = match env::args().nth(1) {
        Some(dir) => PathBuf::from(dir),
        None => PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("ui")
            .join("src")
            .join("lib")
            .join("assets")
            .join("textures"),
    };
    fs::create_dir_all(&out_dir)?;

    let plinth_path = out_dir.join("plinth.png");
    let grain_path = out_dir.join("grain.png");
    let card_path = out_dir.join("card.png");

    let plinth = render_plinth(720, 1040, 11);
    write_best_rgb_png(&plinth, &plinth_path)?;

    let grain = render_grain(720, 1040, 260, 11_001);
    write_best_rgba_png(&grain, &grain_path)?;

    let card = render_card(656, 240, 11_002);
    write_best_rgb_png(&card, &card_path)?;

    let mut total = 0_u64;
    for path in [&plinth_path, &grain_path, &card_path] {
        let size = fs::metadata(path)?.len();
        total += size;
        println!("{}: {} bytes", path.display(), size);
    }
    println!(
        "total: {} bytes ({:.2} MB)",
        total,
        total as f64 / (1024.0 * 1024.0)
    );
    Ok(())
}
