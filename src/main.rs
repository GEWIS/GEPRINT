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
    let mut copies = use_signal(|| 1u32);
    let mut two_sided = use_signal(|| false);
    let mut preview_url = use_signal(String::new);
    let mut preview_kind = use_signal(String::new); // "image" | "pdf" | ""

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
                                    // Build a preview object-URL in the browser from the picked
                                    // file, so we never ship the bytes back down just to show them.
                                    let mime = f.content_type().unwrap_or_default();
                                    let kind = if mime.starts_with("image/") {
                                        "image"
                                    } else if mime == "application/pdf" {
                                        "pdf"
                                    } else {
                                        ""
                                    };
                                    if !kind.is_empty() {
                                        if let Ok(url) = document::eval(
                                            "const el = document.getElementById('file-input');\
                                             const f = el && el.files && el.files[0];\
                                             if (!f) return '';\
                                             if (window.__gp_url) URL.revokeObjectURL(window.__gp_url);\
                                             window.__gp_url = URL.createObjectURL(f);\
                                             return window.__gp_url;",
                                        )
                                        .await
                                        {
                                            preview_url.set(url.as_str().unwrap_or("").to_string());
                                            preview_kind.set(kind.to_string());
                                        }
                                    } else {
                                        preview_url.set(String::new());
                                        preview_kind.set(String::new());
                                    }
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
                                preview_url.set(String::new());
                                preview_kind.set(String::new());
                                // Reset the native input (so re-selecting the same file fires
                                // onchange) and release the preview object-URL.
                                document::eval(
                                    "document.getElementById('file-input').value = '';\
                                     if (window.__gp_url) { URL.revokeObjectURL(window.__gp_url); window.__gp_url = null; }",
                                );
                            },
                            "✕"
                        }
                    }
                }
            }

            section {
                label { "Copies" }
                input {
                    class: "num",
                    r#type: "number",
                    min: "1",
                    max: "999",
                    value: "{copies}",
                    onchange: move |e| {
                        let n = e.value().parse::<u32>().unwrap_or(1).clamp(1, 999);
                        copies.set(n);
                    },
                }
                label { class: "check",
                    input {
                        r#type: "checkbox",
                        checked: two_sided(),
                        onchange: move |e| two_sided.set(e.checked()),
                    }
                    "Print on both sides"
                }
            }

            if !preview_url.read().is_empty() {
                section { class: "preview-sec",
                    label { "Preview" }
                    div { class: "preview",
                        if preview_kind() == "image" {
                            img { src: "{preview_url}", alt: "Preview of the selected file" }
                        } else {
                            iframe { class: "pdf", src: "{preview_url}", title: "Preview of the selected file" }
                        }
                    }
                }
            }

            button {
                class: "primary",
                disabled: selected.read().is_empty() || file_bytes.read().is_empty(),
                onclick: move |_| async move {
                    status.set("Printing…".into());
                    let (printer, name, n, duplex, bytes) = (
                        selected.read().clone(),
                        filename.read().clone(),
                        copies(),
                        two_sided(),
                        file_bytes.read().clone(),
                    );
                    let res = print_file(printer, name, n, duplex, bytes).await;
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

/// Print `bytes` (original name `filename`) on `printer` with the given options.
/// Returns the CUPS job id.
#[server]
async fn print_file(
    printer: String,
    filename: String,
    copies: u32,
    two_sided: bool,
    bytes: Vec<u8>,
) -> ServerFnResult<String> {
    server::print(printer, filename, copies, two_sided, bytes)
        .await
        .map_err(ServerFnError::new)
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

    pub async fn print(
        printer: String,
        filename: String,
        copies: u32,
        two_sided: bool,
        bytes: Vec<u8>,
    ) -> Result<String, String> {
        if !valid_printer(&printer) {
            return Err("invalid printer name".into());
        }
        if bytes.is_empty() {
            return Err("empty file".into());
        }
        let copies = copies.clamp(1, 999);

        let title = if filename.is_empty() { "geprint" } else { &filename };
        let sides = if two_sided { "two-sided-long-edge" } else { "one-sided" };

        // Pipe the bytes straight into `lp` via stdin: no temp file to race on
        // deletion, no filename-extension format sniffing. `lp` with no file
        // argument reads the job from stdin.
        let mut child = Command::new("lp")
            .arg("-d")
            .arg(&printer)
            .arg("-t")
            .arg(title)
            .arg("-n")
            .arg(copies.to_string())
            .arg("-o")
            .arg(format!("sides={sides}"))
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("failed to run lp: {e}"))?;

        {
            use tokio::io::AsyncWriteExt;
            let mut stdin = child.stdin.take().ok_or("failed to open lp stdin")?;
            stdin
                .write_all(&bytes)
                .await
                .map_err(|e| format!("failed to write to lp: {e}"))?;
            stdin.shutdown().await.ok();
        }

        let out = child
            .wait_with_output()
            .await
            .map_err(|e| format!("failed to wait for lp: {e}"))?;

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
.browse { flex: 1; text-align: center; padding: .4rem .85rem; border-radius: .4rem; cursor: pointer;
  font-weight: 600; color: #fff; background: #2a2f3a; border: 1px solid #3a4150; transition: background .15s; }
.browse:hover { background: #3a4150; }
.file-row:has(.fname) .browse { flex: 0 0 auto; text-align: left; }
.fname { flex: 1; min-width: 0; color: #c7ccd6; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.clear { flex: 0 0 auto; width: 2rem; height: 2rem; display: inline-flex; align-items: center;
  justify-content: center; padding: 0; border-radius: .4rem; line-height: 1; font-size: 1rem;
  color: #9aa0aa; background: transparent; border: 1px solid #2a2f3a; }
.clear:hover { color: #fff; background: #d8232a; border-color: #d8232a; }
select { flex: 1; min-width: 12rem; padding: .5rem .6rem; border-radius: .5rem;
  border: 1px solid #2a2f3a; background: #171a21; color: #e6e6e6; }
button { padding: .55rem 1rem; border-radius: .5rem; border: 0; cursor: pointer; font-weight: 600; }
button:disabled { opacity: .45; cursor: not-allowed; }
.primary { background: #d8232a; color: #fff; width: 100%; margin-top: .5rem; padding: .7rem; font-size: 1.05rem; }
.ghost { background: transparent; color: #9aa0aa; border: 1px solid #2a2f3a; }
.status { margin-top: 1rem; padding: .7rem .9rem; border-radius: .5rem; background: #171a21; border: 1px solid #2a2f3a; }
.num { width: 5rem; padding: .5rem .6rem; border-radius: .5rem; border: 1px solid #2a2f3a;
  background: #171a21; color: #e6e6e6; }
.check { min-width: 0; font-weight: 400; display: inline-flex; align-items: center; gap: .4rem;
  color: #c7ccd6; cursor: pointer; }
.check input { width: 1.05rem; height: 1.05rem; accent-color: #d8232a; cursor: pointer; }
.preview-sec { flex-direction: column; align-items: stretch; }
.preview { border: 1px solid #2a2f3a; border-radius: .5rem; overflow: hidden; background: #0b0d11; }
.preview img { display: block; max-width: 100%; max-height: 26rem; margin: 0 auto; }
.preview .pdf { display: block; width: 100%; height: 26rem; border: 0; }
.warn { color: #e6b800; } .err, .status:has(+ *) { color: #ff6b6b; }
"#;
