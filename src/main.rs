use dioxus::prelude::*;

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let mut printers = use_resource(list_printers);
    let mut selected = use_signal(String::new);
    let mut filename = use_signal(String::new);
    let mut file_bytes = use_signal(Vec::<u8>::new);
    let mut status = use_signal(String::new);

    rsx! {
        document::Title { "GEPRINT" }
        style { {CSS} }
        main {
            h1 { title: "GEWIS Ervaart Papier Rijk In Nieuwe Teksten", "GEPRINT" }
            p { class: "sub", "Upload a file and send it to a printer." }

            section {
                label { "Printer" }
                match &*printers.read() {
                    Some(Ok(list)) if !list.is_empty() => rsx! {
                        select {
                            value: "{selected}",
                            onchange: move |e| selected.set(e.value()),
                            option { value: "", disabled: true, selected: selected.read().is_empty(), "Select a printer…" }
                            for p in list.clone() {
                                option { value: "{p}", "{p}" }
                            }
                        }
                    },
                    Some(Ok(_)) => rsx! { p { class: "warn", "No printers found." } },
                    Some(Err(e)) => rsx! { p { class: "err", "Failed to list printers: {e}" } },
                    None => rsx! { p { "Loading printers…" } },
                }
                button { class: "ghost", onclick: move |_| printers.restart(), "↻ Refresh" }
            }

            section {
                label { "File" }
                div { class: "file-row",
                    label { class: "browse", r#for: "file-input", "Choose file" }
                    input {
                        id: "file-input",
                        class: "file-hidden",
                        r#type: "file",
                        onchange: move |e: FormEvent| async move {
                            if let Some(f) = e.files().into_iter().next() {
                                if let Ok(bytes) = f.read_bytes().await {
                                    filename.set(f.name());
                                    file_bytes.set(bytes.to_vec());
                                }
                            }
                        },
                    }
                    if !filename.read().is_empty() {
                        span { class: "fname", "{filename}" }
                        button {
                            r#type: "button",
                            class: "clear",
                            title: "Remove file",
                            onclick: move |_| {
                                filename.set(String::new());
                                file_bytes.set(Vec::new());
                                // Reset the native input so re-selecting the same file fires onchange.
                                document::eval("document.getElementById('file-input').value = ''");
                            },
                            "✕"
                        }
                    }
                }
            }

            button {
                class: "primary",
                disabled: selected.read().is_empty() || file_bytes.read().is_empty(),
                onclick: move |_| async move {
                    status.set("Printing…".into());
                    let res = print_file(
                        selected.read().clone(),
                        filename.read().clone(),
                        file_bytes.read().clone(),
                    ).await;
                    match res {
                        Ok(job) => status.set(format!("✓ Submitted: {job}")),
                        Err(e) => status.set(format!("✗ {e}")),
                    }
                },
                "Print"
            }

            if !status.read().is_empty() {
                p { class: "status", "{status}" }
            }
        }
    }
}

/// List all CUPS printer names (`lpstat -e`).
#[server]
async fn list_printers() -> ServerFnResult<Vec<String>> {
    server::printer_names().await.map_err(ServerFnError::new)
}

/// Print `bytes` (original name `filename`) on `printer`. Returns the CUPS job id.
#[server]
async fn print_file(
    printer: String,
    filename: String,
    bytes: Vec<u8>,
) -> ServerFnResult<String> {
    server::print(printer, filename, bytes).await.map_err(ServerFnError::new)
}

#[cfg(feature = "server")]
mod server {
    use tokio::process::Command;

    /// Reject anything that isn't a plausible CUPS queue name so it can never be
    /// smuggled into the `lp -d` argument as flags/paths.
    fn valid_printer(name: &str) -> bool {
        !name.is_empty()
            && name.len() <= 127
            && name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
    }

    pub async fn printer_names() -> Result<Vec<String>, String> {
        let out = Command::new("lpstat")
            .arg("-e")
            .output()
            .await
            .map_err(|e| format!("failed to run lpstat: {e}"))?;
        if !out.status.success() {
            return Err(format!(
                "lpstat exited {}: {}",
                out.status,
                String::from_utf8_lossy(&out.stderr).trim()
            ));
        }
        Ok(String::from_utf8_lossy(&out.stdout)
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect())
    }

    pub async fn print(printer: String, filename: String, bytes: Vec<u8>) -> Result<String, String> {
        if !valid_printer(&printer) {
            return Err("invalid printer name".into());
        }
        if bytes.is_empty() {
            return Err("empty file".into());
        }

        // Temp file, print, delete — NamedTempFile removes on drop.
        let tmp = tokio::task::spawn_blocking(move || -> Result<_, String> {
            use std::io::Write;
            let mut f = tempfile::NamedTempFile::new().map_err(|e| e.to_string())?;
            f.write_all(&bytes).map_err(|e| e.to_string())?;
            f.flush().map_err(|e| e.to_string())?;
            Ok(f)
        })
        .await
        .map_err(|e| e.to_string())??;

        let title = if filename.is_empty() { "geprint" } else { &filename };
        let out = Command::new("lp")
            .arg("-d")
            .arg(&printer)
            .arg("-t")
            .arg(title)
            .arg("--")
            .arg(tmp.path())
            .output()
            .await
            .map_err(|e| format!("failed to run lp: {e}"))?;

        if !out.status.success() {
            return Err(format!(
                "lp exited {}: {}",
                out.status,
                String::from_utf8_lossy(&out.stderr).trim()
            ));
        }
        // lp prints e.g. "request id is Office-42 (1 file(s))".
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }
}

const CSS: &str = r#"
* { box-sizing: border-box; }
body { margin: 0; font: 16px/1.5 system-ui, sans-serif; background: #0f1115; color: #e6e6e6; }
main { max-width: 34rem; margin: 3rem auto; padding: 0 1.25rem; }
h1 { margin: 0 0 .25rem; font-size: 2rem; letter-spacing: -.02em; }
h1[title] { cursor: help; }
.sub { color: #9aa0aa; margin: .25rem 0 1.5rem; }
section { margin: 1.25rem 0; display: flex; flex-wrap: wrap; align-items: center; gap: .6rem; }
label { min-width: 4rem; font-weight: 600; }
.file-row { flex: 1; min-width: 12rem; display: flex; align-items: center; gap: .6rem;
  padding: .4rem .5rem; border-radius: .5rem; border: 1px solid #2a2f3a; background: #171a21; }
.file-hidden { position: absolute; width: 1px; height: 1px; padding: 0; margin: -1px;
  overflow: hidden; clip: rect(0 0 0 0); border: 0; }
.browse { min-width: 0; flex: 0 0 auto; padding: .4rem .85rem; border-radius: .4rem; cursor: pointer;
  font-weight: 600; color: #fff; background: #2a2f3a; border: 1px solid #3a4150; transition: background .15s; }
.browse:hover { background: #3a4150; }
.fname { flex: 1; min-width: 0; color: #c7ccd6; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.clear { flex: 0 0 auto; padding: .25rem .55rem; border-radius: .4rem; line-height: 1;
  color: #9aa0aa; background: transparent; border: 1px solid #2a2f3a; }
.clear:hover { color: #fff; background: #d8232a; border-color: #d8232a; }
select { flex: 1; min-width: 12rem; padding: .5rem .6rem; border-radius: .5rem;
  border: 1px solid #2a2f3a; background: #171a21; color: #e6e6e6; }
button { padding: .55rem 1rem; border-radius: .5rem; border: 0; cursor: pointer; font-weight: 600; }
button:disabled { opacity: .45; cursor: not-allowed; }
.primary { background: #d8232a; color: #fff; width: 100%; margin-top: .5rem; padding: .7rem; font-size: 1.05rem; }
.ghost { background: transparent; color: #9aa0aa; border: 1px solid #2a2f3a; }
.status { margin-top: 1rem; padding: .7rem .9rem; border-radius: .5rem; background: #171a21; border: 1px solid #2a2f3a; }
.warn { color: #e6b800; } .err, .status:has(+ *) { color: #ff6b6b; }
"#;
