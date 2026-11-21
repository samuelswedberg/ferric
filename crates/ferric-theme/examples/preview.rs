use std::{env, fs, path::PathBuf};

use ferric_core::model::Art;
use ferric_theme::extract;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cover = env::args().nth(1).ok_or("usage: preview <cover.jpg>")?;
    let bytes = fs::read(&cover)?;
    let key = PathBuf::from(&cover)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("preview")
        .to_owned();
    let cache = PathBuf::from("preview-cache");
    let theme = extract(&bytes, &key, &cache)?;

    let sleeve = match &theme.sleeve {
        Some(Art::File { path }) => path.replace('\\', "/"),
        _ => String::new(),
    };
    let record = match &theme.record {
        Some(Art::File { path }) => path.replace('\\', "/"),
        _ => String::new(),
    };

    let textures_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("ui")
        .join("src")
        .join("lib")
        .join("assets")
        .join("textures");
    let plinth = textures_dir
        .join("plinth.png")
        .to_string_lossy()
        .replace('\\', "/");
    let grain = textures_dir
        .join("grain.png")
        .to_string_lossy()
        .replace('\\', "/");

    let html = format!(
        r#"<!doctype html><meta charset="utf-8"><title>Ferric theme preview — {key}</title>
<style>
  @keyframes spin {{ from {{ transform: rotate(0deg); }} to {{ transform: rotate(360deg); }} }}
  body {{ margin: 0; min-height: 100vh; display: grid; place-items: center; background: #111; color: #EDE4D4; font-family: Bahnschrift, "Segoe UI", sans-serif; }}
  .window {{ position: relative; width: 360px; height: 520px; border-radius: 20px; overflow: hidden; background: #1E1B18 url('{plinth}') center/cover; box-shadow: 0 24px 60px rgba(0,0,0,.6); }}
  .record-wrap {{ position: absolute; left: 130px; top: 34px; width: 212px; height: 212px; }}
  .record {{ width: 100%; height: 100%; border-radius: 50%; animation: spin 1.8s linear infinite; display: block; }}
  .rim {{ position: absolute; inset: 0; border-radius: 50%; border: 1.5px solid rgba(255,255,255,0.2); pointer-events: none; }}
  .sleeve {{ position: absolute; left: 16px; top: 22px; width: 232px; height: 232px; transform: rotate(-2deg); box-shadow: 6px 8px 14px rgba(0,0,0,0.7); object-fit: cover; }}
  .meta {{ position: absolute; left: 20px; right: 20px; top: 268px; }}
  .title {{ margin: 0; font-size: 24px; font-weight: 600; }}
  .artist {{ margin: 4px 0 0; font-size: 14px; color: #B8AC98; }}
  .bar {{ position: absolute; left: 20px; top: 334px; width: 320px; height: 6px; border-radius: 3px; background: #0D0B0A; overflow: hidden; }}
  .fill {{ width: 42%; height: 100%; background: rgb({ar},{ag},{ab}); }}
  .swatch {{ position: absolute; left: 20px; top: 360px; padding: 10px 14px; border-radius: 6px; font-weight: 600; color: rgb({or},{og},{ob}); background: rgb({ar},{ag},{ab}); }}
  .grain {{ position: absolute; inset: 0; background: url('{grain}') center/cover; opacity: 0.2; pointer-events: none; }}
</style>
<main class="window">
  <div class="record-wrap">
    <img class="record" src="{record}" alt="record">
    <div class="rim"></div>
  </div>
  <img class="sleeve" src="{sleeve}" alt="sleeve">
  <div class="meta">
    <h1 class="title">{key}</h1>
    <p class="artist">Accent: rgb({ar}, {ag}, {ab})</p>
  </div>
  <div class="bar"><div class="fill"></div></div>
  <div class="swatch">on_accent sample</div>
  <div class="grain"></div>
</main>"#,
        ar = theme.accent.r,
        ag = theme.accent.g,
        ab = theme.accent.b,
        or = theme.on_accent.r,
        og = theme.on_accent.g,
        ob = theme.on_accent.b,
    );
    fs::write("preview.html", html)?;
    println!("wrote preview.html (sleeve: {sleeve}, record: {record})");
    Ok(())
}
