use std::{env, fs, path::PathBuf};

use ferric_core::model::Art;
use ferric_theme::extract;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cover = env::args().nth(1).ok_or("usage: preview <cover.jpg>")?;
    let bytes = fs::read(&cover)?;
    let cache = PathBuf::from("preview-cache");
    let theme = extract(&bytes, "preview", &cache)?;
    let background = match &theme.background {
        Some(Art::File { path }) => path.replace('\\', "/"),
        _ => String::new(),
    };
    let html = format!(
        r#"<!doctype html><meta charset="utf-8"><title>Ferric theme preview</title>
<style>
body {{ margin: 0; min-height: 100vh; display: grid; place-items: center; background: #111; font-family: "Segoe UI", sans-serif; }}
.card {{ box-sizing: border-box; width: 360px; height: 520px; padding: 28px; color: rgb({tr},{tg},{tb}); background: url('{background}') center/cover; border-radius: 12px; }}
.accent {{ margin-top: 300px; padding: 14px; color: rgb({or},{og},{ob}); background: rgb({ar},{ag},{ab}); border-radius: 8px; }}
.muted {{ color: rgb({mr},{mg},{mb}); }}
</style><main class="card"><div class="accent">Accent and on-accent</div><h2>Sample track</h2><p class="muted">Sample artist</p></main>"#,
        tr = theme.text.r,
        tg = theme.text.g,
        tb = theme.text.b,
        or = theme.on_accent.r,
        og = theme.on_accent.g,
        ob = theme.on_accent.b,
        ar = theme.accent.r,
        ag = theme.accent.g,
        ab = theme.accent.b,
        mr = theme.text_muted.r,
        mg = theme.text_muted.g,
        mb = theme.text_muted.b,
    );
    fs::write("preview.html", html)?;
    Ok(())
}
