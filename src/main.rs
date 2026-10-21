// MD Lite : lecteur et éditeur Markdown léger pour Windows.
// Une fenêtre native avec WebView2 affiche l'éditeur (md-lite.html, embarqué dans l'exe).
// La page parle au programme via le protocole interne https://mdlite.localhost/ :
// lecture et écriture des fichiers (un état par onglet), boîtes de dialogue Windows,
// images relatives, paramètres, association des fichiers .md.

#![windows_subsystem = "windows"]

use std::borrow::Cow;
use std::cell::Cell;
use std::collections::HashMap;
use std::ffi::OsString;
use std::fs;
use std::hash::{BuildHasher, Hasher};
use std::io;
use std::num::NonZeroIsize;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::SystemTime;

use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawWindowHandle, Win32WindowHandle, WindowHandle,
};
use rfd::{FileDialog, MessageButtons, MessageDialog, MessageDialogResult, MessageLevel};
use serde_json::{json, Value};
use tao::dpi::LogicalSize;
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy};
use tao::platform::windows::{IconExtWindows, WindowBuilderExtWindows, WindowExtWindows};
use tao::window::{Icon, Theme, Window, WindowBuilder};
use windows_sys::core::BOOL;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::DataExchange::COPYDATASTRUCT;
use windows_sys::Win32::UI::Shell::{
    DefSubclassProc, SHChangeNotify, SHOpenWithDialog, SetWindowSubclass, ShellExecuteW, OAIF_ALLOW_REGISTRATION,
    OAIF_REGISTER_EXT, OPENASINFO, SHCNE_ASSOCCHANGED, SHCNF_IDLIST,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetPropW, IsIconic, SendMessageW, SetForegroundWindow, SetPropW, ShowWindow, SW_RESTORE,
    SW_SHOWNORMAL, WM_COPYDATA,
};
use winreg::enums::{HKEY_CURRENT_USER, KEY_SET_VALUE, REG_NONE};
use winreg::{RegKey, RegValue};
use wry::http::{header, Request, Response};
use wry::{DragDropEvent, NewWindowResponse, WebContext, WebViewBuilder, WebViewBuilderExtWindows};

const PAGE: &str = include_str!("../md-lite.html");
const ORIGIN: &str = "https://mdlite.localhost";
const PROG_ID: &str = "MDLite.Markdown";
const EXTS: [&str; 2] = [".md", ".markdown"];
// Marque posée sur nos fenêtres pour qu'un second lancement les retrouve.
const WINDOW_PROP: &str = "MDLite.Window";
const COPY_ID: usize = 0x4D_444C;

type Res = Response<Cow<'static, [u8]>>;
type Docs = Mutex<HashMap<String, Doc>>;

#[derive(Clone, Copy, Default)]
enum Enc {
    #[default]
    Utf8,
    Utf8Bom,
    Utf16,
    Ansi,
}

#[derive(Default)]
struct Doc {
    path: Option<PathBuf>,
    enc: Enc,
    crlf: bool,
    mtime: Option<SystemTime>,
}

enum UserEvent {
    Title(String),
    Open(Vec<PathBuf>),
    Theme(Option<Theme>),
    Close,
}

static PROXY: OnceLock<Mutex<EventLoopProxy<UserEvent>>> = OnceLock::new();

fn main() {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    let flag = |f: &str| args.iter().any(|a| a == f);
    let quiet = flag("--quiet");
    if flag("--install") {
        return report(install(), quiet, "MD Lite est installé. Ouvre MD Lite et clique sur « Définir par défaut », ou passe par clic droit sur un .md > Ouvrir avec.");
    }
    if flag("--uninstall") {
        return report(uninstall(), quiet, "MD Lite est désinstallé.");
    }
    heal_registration();
    let files: Vec<PathBuf> = args
        .iter()
        .filter(|a| !a.to_string_lossy().starts_with("--"))
        .map(|a| std::path::absolute(a).unwrap_or_else(|_| PathBuf::from(a)))
        .collect();
    let fresh = flag("--new");
    let in_window = settings()["openIn"].as_str() == Some("window");
    if !files.is_empty() && !fresh && !in_window && hand_off(&files) {
        return;
    }
    run(files, fresh);
}

fn run(files: Vec<PathBuf>, fresh: bool) {
    let docs: Arc<Docs> = Arc::new(Mutex::new(HashMap::new()));
    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let _ = PROXY.set(Mutex::new(event_loop.create_proxy()));
    let theme = theme_setting();
    let dark = theme.map(|t| t == Theme::Dark).unwrap_or_else(is_dark);
    let (size, maximized) = load_window_state();
    let window = WindowBuilder::new()
        .with_title("MD Lite")
        .with_inner_size(size)
        .with_maximized(maximized)
        .with_theme(Some(if dark { Theme::Dark } else { Theme::Light }))
        .with_window_icon(Icon::from_resource(1, None).ok())
        .with_taskbar_icon(Icon::from_resource(1, None).ok())
        .build(&event_loop)
        .expect("création de la fenêtre");
    let hwnd = window.hwnd();
    unsafe {
        let prop = wide(WINDOW_PROP);
        SetPropW(hwnd as HWND, prop.as_ptr(), 1 as _);
        SetWindowSubclass(hwnd as HWND, Some(subclass), 1, 0);
    }
    let nonce = make_nonce();
    let startup = Arc::new(json!({
        "files": files.iter().map(|p| p.to_string_lossy()).collect::<Vec<_>>(),
        "fresh": fresh,
    }));

    let mut context = WebContext::new(Some(app_dir().join("WebView2")));
    let api_proxy = event_loop.create_proxy();
    let drop_proxy = event_loop.create_proxy();
    let title_proxy = event_loop.create_proxy();
    let dragging = Cell::new(false);
    let built = WebViewBuilder::new_with_web_context(&mut context)
        .with_url(format!("{ORIGIN}/"))
        .with_https_scheme(true)
        .with_browser_accelerator_keys(false)
        .with_devtools(cfg!(debug_assertions))
        .with_background_color(if dark { (13, 17, 23, 255) } else { (255, 255, 255, 255) })
        .with_asynchronous_custom_protocol("mdlite".into(), move |_id, req, responder| {
            let (docs, proxy, nonce, startup) = (docs.clone(), api_proxy.clone(), nonce.clone(), startup.clone());
            std::thread::spawn(move || {
                let ctx = Ctx { docs: &docs, hwnd, proxy: &proxy, nonce: &nonce, startup: &startup };
                responder.respond(route(req, &ctx).unwrap_or_else(|e| text(500, &e.to_string())))
            });
        })
        .with_drag_drop_handler(move |event| match event {
            DragDropEvent::Enter { paths, .. } => {
                dragging.set(!paths.is_empty());
                dragging.get()
            }
            DragDropEvent::Over { .. } => dragging.get(),
            DragDropEvent::Drop { paths, .. } if !paths.is_empty() => {
                dragging.set(false);
                let _ = drop_proxy.send_event(UserEvent::Open(paths));
                true
            }
            _ => {
                dragging.set(false);
                false
            }
        })
        .with_document_title_changed_handler(move |t| {
            let _ = title_proxy.send_event(UserEvent::Title(t));
        })
        .with_new_window_req_handler(|url, _| {
            open_url(&url);
            NewWindowResponse::Deny
        })
        .with_navigation_handler(|url| {
            if url.starts_with(ORIGIN) {
                return true;
            }
            open_url(&url);
            false
        })
        .build(&window);
    let webview = match built {
        Ok(w) => w,
        Err(e) => {
            MessageDialog::new()
                .set_title("MD Lite")
                .set_level(MessageLevel::Error)
                .set_description(format!(
                    "Impossible d'afficher l'éditeur : {e}\n\nMD Lite a besoin du runtime Microsoft WebView2 (fourni avec Windows 10/11 à jour) : https://developer.microsoft.com/microsoft-edge/webview2/"
                ))
                .show();
            return;
        }
    };

    let close_proxy = event_loop.create_proxy();
    event_loop.run(move |event, _, flow| {
        *flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                // La page décide (onglets non enregistrés) puis appelle /api/close.
                let p = close_proxy.clone();
                let asked = webview.evaluate_script_with_callback(
                    "window.__mdlite ? __mdlite.requestClose() : false",
                    move |res| {
                        if res != "true" {
                            let _ = p.send_event(UserEvent::Close);
                        }
                    },
                );
                if asked.is_err() {
                    *flow = ControlFlow::Exit;
                }
            }
            Event::UserEvent(UserEvent::Title(t)) => window.set_title(&t),
            Event::UserEvent(UserEvent::Open(paths)) => {
                let list: Vec<String> = paths.iter().map(|p| js_string(&p.to_string_lossy())).collect();
                let _ = webview.evaluate_script(&format!("window.__mdlite && __mdlite.openPaths([{}])", list.join(",")));
            }
            Event::UserEvent(UserEvent::Theme(t)) => {
                let dark = t.map(|t| t == Theme::Dark).unwrap_or_else(is_dark);
                window.set_theme(Some(if dark { Theme::Dark } else { Theme::Light }));
            }
            Event::UserEvent(UserEvent::Close) => {
                save_window_state(&window);
                *flow = ControlFlow::Exit;
            }
            _ => {}
        }
    });
}

// ---------- Instance unique ----------

// Reçoit les fichiers envoyés par un second lancement de MD Lite (double-clic dans l'Explorateur).
unsafe extern "system" fn subclass(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM, _id: usize, _data: usize) -> LRESULT {
    if msg == WM_COPYDATA {
        let cds = unsafe { &*(lp as *const COPYDATASTRUCT) };
        if cds.dwData == COPY_ID && !cds.lpData.is_null() {
            let units = unsafe { std::slice::from_raw_parts(cds.lpData as *const u16, cds.cbData as usize / 2) };
            let paths = String::from_utf16_lossy(units).split('\n').filter(|s| !s.is_empty()).map(PathBuf::from).collect();
            if let Some(p) = PROXY.get() {
                let _ = p.lock().unwrap().send_event(UserEvent::Open(paths));
            }
            return 1;
        }
    }
    unsafe { DefSubclassProc(hwnd, msg, wp, lp) }
}

// Confie les fichiers à une fenêtre MD Lite déjà ouverte (la plus récemment utilisée).
fn hand_off(files: &[PathBuf]) -> bool {
    unsafe extern "system" fn find(h: HWND, out: LPARAM) -> BOOL {
        let prop = wide(WINDOW_PROP);
        if unsafe { !GetPropW(h, prop.as_ptr()).is_null() } {
            unsafe { *(out as *mut HWND) = h };
            return 0;
        }
        1
    }
    let mut target: HWND = std::ptr::null_mut();
    unsafe { EnumWindows(Some(find), &mut target as *mut HWND as LPARAM) };
    if target.is_null() {
        return false;
    }
    let list: Vec<String> = files.iter().map(|p| p.to_string_lossy().into_owned()).collect();
    let data: Vec<u16> = list.join("\n").encode_utf16().collect();
    let cds = COPYDATASTRUCT { dwData: COPY_ID, cbData: (data.len() * 2) as u32, lpData: data.as_ptr() as *mut _ };
    unsafe {
        if IsIconic(target) != 0 {
            ShowWindow(target, SW_RESTORE);
        }
        SetForegroundWindow(target);
        SendMessageW(target, WM_COPYDATA, 0, &cds as *const COPYDATASTRUCT as LPARAM) != 0
    }
}

// ---------- Protocole interne ----------

struct Ctx<'a> {
    docs: &'a Docs,
    hwnd: isize,
    proxy: &'a EventLoopProxy<UserEvent>,
    nonce: &'a str,
    startup: &'a Value,
}

fn route(req: Request<Vec<u8>>, c: &Ctx) -> io::Result<Res> {
    let path = req.uri().path().to_string();
    let q = |k: &str| query(req.uri().query(), k);
    let id = q("doc").unwrap_or_default();
    let body = || String::from_utf8_lossy(req.body()).into_owned();
    let put = req.method().as_str() == "PUT";

    // Fichiers liés au document : /doc/<onglet>/<chemin relatif>, /root/<onglet>/<chemin depuis la racine git>.
    for (prefix, root) in [("/doc/", false), ("/root/", true)] {
        if let Some(rest) = path.strip_prefix(prefix) {
            let (doc, rel) = rest.split_once('/').unwrap_or((rest, ""));
            let rel = pct_decode(rel);
            let rel = if root { format!("/{rel}") } else { rel };
            return Ok(serve(resolve(c.docs, doc, &rel)));
        }
    }
    if let Some(rest) = path.strip_prefix("/abs/") {
        return Ok(serve(Some(PathBuf::from(pct_decode(rest)))));
    }

    match path.as_str() {
        "/" => Ok(page(c.nonce)),
        "/api/startup" => Ok(respond(200, "application/json", c.startup.to_string().into_bytes())),
        "/api/store" => {
            let key = q("key").unwrap_or_default();
            if key != "settings" && key != "session" {
                return Ok(empty(400));
            }
            let file = app_dir().join(format!("{key}.json"));
            if put {
                fs::create_dir_all(app_dir())?;
                fs::write(file, body())?;
                return Ok(empty(204));
            }
            Ok(text(200, &fs::read_to_string(file).unwrap_or_default()))
        }
        "/api/theme" => {
            let t = match q("value").as_deref() {
                Some("dark") => Some(Theme::Dark),
                Some("light") => Some(Theme::Light),
                _ => None,
            };
            let _ = c.proxy.send_event(UserEvent::Theme(t));
            Ok(empty(204))
        }
        "/api/file" if put => {
            let mut docs = c.docs.lock().unwrap();
            let d = docs.entry(id).or_default();
            if d.path.is_none() {
                return Ok(empty(409));
            }
            write_doc(d, &body())?;
            Ok(empty(204))
        }
        "/api/file" => {
            let mut docs = c.docs.lock().unwrap();
            let d = docs.entry(id).or_default();
            let Some(p) = d.path.clone() else { return Ok(empty(204)) };
            let t = read_doc(d, &p)?;
            Ok(doc_response(d, t))
        }
        "/api/pick" => {
            let mut dlg = md_filters(FileDialog::new()).set_parent(&Parent(c.hwnd));
            if let Some(dir) = doc_dir(c.docs, &id) {
                dlg = dlg.set_directory(dir);
            }
            let files = dlg.pick_files().unwrap_or_default();
            let list: Vec<String> = files.iter().map(|p| p.to_string_lossy().into_owned()).collect();
            Ok(respond(200, "application/json", json!(list).to_string().into_bytes()))
        }
        "/api/open" => {
            let Some(p) = q("path").map(PathBuf::from) else { return Ok(empty(400)) };
            let mut d = Doc::default();
            let t = read_doc(&mut d, &p)?;
            d.path = Some(p);
            let r = doc_response(&d, t);
            c.docs.lock().unwrap().insert(id, d);
            Ok(r)
        }
        "/api/saveas" => {
            let name = q("name").filter(|n| !n.is_empty()).unwrap_or_else(|| "sans-titre.md".into());
            let mut dlg = md_filters(FileDialog::new()).set_file_name(name).set_parent(&Parent(c.hwnd));
            if let Some(dir) = doc_dir(c.docs, &id) {
                dlg = dlg.set_directory(dir);
            }
            let Some(p) = dlg.save_file() else { return Ok(empty(204)) };
            let mut docs = c.docs.lock().unwrap();
            let d = docs.entry(id).or_default();
            if d.path.is_none() {
                // Nouveau fichier : fins de ligne choisies dans les paramètres.
                d.crlf = q("crlf").as_deref() == Some("1");
            }
            d.path = Some(p);
            write_doc(d, &body())?;
            Ok(doc_response(d, String::new()))
        }
        "/api/closedoc" => {
            c.docs.lock().unwrap().remove(&id);
            Ok(empty(204))
        }
        "/api/changed" => {
            let docs = c.docs.lock().unwrap();
            let Some(d) = docs.get(&id) else { return Ok(text(200, "0")) };
            let now = d.path.as_ref().and_then(|p| fs::metadata(p).and_then(|m| m.modified()).ok());
            Ok(text(200, if now.is_some() && now != d.mtime { "1" } else { "0" }))
        }
        "/api/ack" => {
            if let Some(d) = c.docs.lock().unwrap().get_mut(&id) {
                d.mtime = d.path.as_ref().and_then(|p| fs::metadata(p).and_then(|m| m.modified()).ok());
            }
            Ok(empty(204))
        }
        "/api/confirmclose" => {
            let name = q("name").unwrap_or_default();
            let res = MessageDialog::new()
                .set_title("MD Lite")
                .set_level(MessageLevel::Warning)
                .set_description(format!("Enregistrer les modifications de « {name} » ?"))
                .set_buttons(MessageButtons::YesNoCancelCustom(
                    "Enregistrer".into(),
                    "Ne pas enregistrer".into(),
                    "Annuler".into(),
                ))
                .set_parent(&Parent(c.hwnd))
                .show();
            let answer = match res {
                MessageDialogResult::Yes => "yes",
                MessageDialogResult::No => "no",
                MessageDialogResult::Custom(s) if s == "Enregistrer" => "yes",
                MessageDialogResult::Custom(s) if s == "Ne pas enregistrer" => "no",
                _ => "cancel",
            };
            Ok(text(200, answer))
        }
        "/api/close" => {
            let _ = c.proxy.send_event(UserEvent::Close);
            Ok(empty(204))
        }
        "/api/launch" => {
            spawn_window();
            Ok(empty(204))
        }
        "/api/resolve" => match q("rel").and_then(|r| resolve(c.docs, &id, &r)) {
            Some(p) => Ok(text(200, &p.to_string_lossy())),
            None => Ok(empty(404)),
        },
        "/api/reveal" => {
            if let Some(p) = q("rel").and_then(|r| resolve(c.docs, &id, &r)) {
                reveal(&p);
            }
            Ok(empty(204))
        }
        "/api/default" => Ok(text(200, if is_default() { "1" } else { "0" })),
        "/api/makedefault" => {
            install()?;
            open_with_dialog(c.hwnd);
            Ok(text(200, if is_default() { "1" } else { "0" }))
        }
        _ => Ok(empty(404)),
    }
}

fn page(nonce: &str) -> Res {
    let html = PAGE.replace("<script>", &format!("<script nonce=\"{nonce}\">"));
    // Seuls nos propres scripts s'exécutent, même si un document contient du HTML piégé.
    let csp = format!(
        "default-src 'self'; script-src 'nonce-{nonce}'; style-src 'self' 'unsafe-inline'; img-src * data: blob:; media-src *; font-src * data:; connect-src 'self'; object-src 'none'; frame-src 'none'; base-uri 'none'; form-action 'none'"
    );
    Response::builder()
        .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
        .header("Content-Security-Policy", csp)
        .body(Cow::Owned(html.into_bytes()))
        .unwrap()
}

fn respond(code: u16, kind: &str, data: Vec<u8>) -> Res {
    Response::builder()
        .status(code)
        .header(header::CONTENT_TYPE, kind)
        .header(header::CACHE_CONTROL, "no-store")
        .body(Cow::Owned(data))
        .unwrap()
}

fn empty(code: u16) -> Res {
    respond(code, "text/plain", Vec::new())
}

fn text(code: u16, t: &str) -> Res {
    respond(code, "text/plain; charset=utf-8", t.as_bytes().to_vec())
}

fn doc_response(d: &Doc, t: String) -> Res {
    let p = d.path.as_ref().unwrap();
    let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let mut r = text(200, &t);
    let h = r.headers_mut();
    h.insert("X-Name", header::HeaderValue::from_str(&pct_encode(&name)).unwrap());
    h.insert("X-Path", header::HeaderValue::from_str(&pct_encode(&p.to_string_lossy())).unwrap());
    r
}

fn serve(path: Option<PathBuf>) -> Res {
    let Some(path) = path else { return empty(404) };
    let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    let kind = match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "svg" => "image/svg+xml",
        "bmp" => "image/bmp",
        "ico" => "image/x-icon",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "mp3" => "audio/mpeg",
        "ogg" => "audio/ogg",
        "wav" => "audio/wav",
        _ => "application/octet-stream",
    };
    match fs::read(&path) {
        Ok(data) => respond(200, kind, data),
        Err(_) => empty(404),
    }
}

// Résout un lien du document : relatif au dossier du fichier, "/..." relatif à la racine du dépôt git.
fn resolve(docs: &Docs, id: &str, rel: &str) -> Option<PathBuf> {
    let rel = rel.split(['?', '#']).next().unwrap_or("");
    if rel.is_empty() {
        return None;
    }
    let p = Path::new(rel);
    if p.is_absolute() {
        return Some(p.to_path_buf());
    }
    let dir = doc_dir(docs, id)?;
    if let Some(r) = rel.strip_prefix('/') {
        let root = dir.ancestors().find(|a| a.join(".git").exists()).unwrap_or(&dir).to_path_buf();
        return Some(root.join(r));
    }
    Some(dir.join(rel))
}

fn doc_dir(docs: &Docs, id: &str) -> Option<PathBuf> {
    docs.lock().unwrap().get(id)?.path.as_ref()?.parent().map(Path::to_path_buf)
}

fn md_filters(d: FileDialog) -> FileDialog {
    d.add_filter("Markdown", &["md", "markdown"]).add_filter("Texte", &["txt"]).add_filter("Tous les fichiers", &["*"])
}

// ---------- Lecture / écriture ----------

// Retient l'encodage et les fins de ligne d'origine pour réécrire le fichier à l'identique.
fn read_doc(d: &mut Doc, path: &Path) -> io::Result<String> {
    let b = fs::read(path)?;
    let (t, enc) = if let Some(rest) = b.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        (String::from_utf8_lossy(rest).into_owned(), Enc::Utf8Bom)
    } else if let Some(rest) = b.strip_prefix(&[0xFF, 0xFE]) {
        let units: Vec<u16> = rest.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        (String::from_utf16_lossy(&units), Enc::Utf16)
    } else {
        match String::from_utf8(b) {
            Ok(s) => (s, Enc::Utf8),
            Err(e) => (e.as_bytes().iter().map(|&c| from_cp1252(c)).collect(), Enc::Ansi),
        }
    };
    d.enc = enc;
    d.crlf = t.contains("\r\n");
    d.mtime = fs::metadata(path).and_then(|m| m.modified()).ok();
    Ok(t)
}

fn write_doc(d: &mut Doc, text: &str) -> io::Result<()> {
    let path = d.path.clone().unwrap();
    let mut t = text.replace("\r\n", "\n");
    if d.crlf {
        t = t.replace('\n', "\r\n");
    }
    let bytes = match d.enc {
        Enc::Utf8 => t.into_bytes(),
        Enc::Utf8Bom => [&[0xEF, 0xBB, 0xBF][..], t.as_bytes()].concat(),
        Enc::Utf16 => [0xFF, 0xFE].into_iter().chain(t.encode_utf16().flat_map(u16::to_le_bytes)).collect(),
        // Un caractère absent de Windows-1252 : on passe en UTF-8 plutôt que de le perdre.
        Enc::Ansi => match t.chars().map(to_cp1252).collect::<Option<Vec<u8>>>() {
            Some(b) => b,
            None => {
                d.enc = Enc::Utf8;
                t.into_bytes()
            }
        },
    };
    // Écriture dans un fichier temporaire puis remplacement : jamais de fichier à moitié écrit.
    let name = path.file_name().unwrap().to_string_lossy();
    let tmp = path.with_file_name(format!("{name}.mdlite-tmp"));
    fs::write(&tmp, &bytes)?;
    if let Err(e) = fs::rename(&tmp, &path) {
        let _ = fs::remove_file(&tmp);
        return Err(e);
    }
    d.mtime = fs::metadata(&path).and_then(|m| m.modified()).ok();
    Ok(())
}

const CP1252: [char; 32] = [
    '\u{20AC}', '\u{81}', '\u{201A}', '\u{192}', '\u{201E}', '\u{2026}', '\u{2020}', '\u{2021}', '\u{2C6}', '\u{2030}',
    '\u{160}', '\u{2039}', '\u{152}', '\u{8D}', '\u{17D}', '\u{8F}', '\u{90}', '\u{2018}', '\u{2019}', '\u{201C}',
    '\u{201D}', '\u{2022}', '\u{2013}', '\u{2014}', '\u{2DC}', '\u{2122}', '\u{161}', '\u{203A}', '\u{153}', '\u{9D}',
    '\u{17E}', '\u{178}',
];

fn from_cp1252(b: u8) -> char {
    if (0x80..0xA0).contains(&b) { CP1252[(b - 0x80) as usize] } else { b as char }
}

fn to_cp1252(c: char) -> Option<u8> {
    let u = c as u32;
    if u < 0x80 || (0xA0..0x100).contains(&u) {
        return Some(u as u8);
    }
    CP1252.iter().position(|&x| x == c).map(|i| 0x80 + i as u8)
}

// ---------- Windows ----------

struct Parent(isize);

impl HasWindowHandle for Parent {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        let h = Win32WindowHandle::new(NonZeroIsize::new(self.0).ok_or(HandleError::Unavailable)?);
        Ok(unsafe { WindowHandle::borrow_raw(RawWindowHandle::Win32(h)) })
    }
}

impl HasDisplayHandle for Parent {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        Ok(DisplayHandle::windows())
    }
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

fn shell_open(target: &str) {
    let (op, file) = (wide("open"), wide(target));
    unsafe {
        ShellExecuteW(std::ptr::null_mut(), op.as_ptr(), file.as_ptr(), std::ptr::null(), std::ptr::null(), SW_SHOWNORMAL);
    }
}

// Liens web uniquement : jamais de schéma exotique lancé depuis un document.
fn open_url(url: &str) {
    let l = url.to_ascii_lowercase();
    if l.starts_with("http://") || l.starts_with("https://") || l.starts_with("mailto:") {
        shell_open(url);
    }
}

// Ouvre un fichier lié avec son application ; un exécutable est seulement montré dans l'Explorateur.
fn reveal(p: &Path) {
    const RISKY: [&str; 16] =
        ["exe", "bat", "cmd", "com", "scr", "pif", "ps1", "vbs", "vbe", "js", "jse", "wsf", "msi", "hta", "lnk", "reg"];
    let ext = p.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    if !p.exists() {
        return;
    }
    if p.is_file() && RISKY.contains(&ext.as_str()) {
        let _ = Command::new("explorer").arg(format!("/select,{}", p.display())).spawn();
    } else {
        shell_open(&p.to_string_lossy());
    }
}

fn spawn_window() {
    if let Ok(exe) = std::env::current_exe() {
        let _ = Command::new(exe).arg("--new").spawn();
    }
}

fn app_dir() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    base.join("MDLite")
}

fn settings() -> Value {
    fs::read_to_string(app_dir().join("settings.json"))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(Value::Null)
}

fn theme_setting() -> Option<Theme> {
    match settings()["theme"].as_str() {
        Some("dark") => Some(Theme::Dark),
        Some("light") => Some(Theme::Light),
        _ => None,
    }
}

fn load_window_state() -> (LogicalSize<f64>, bool) {
    let s = fs::read_to_string(app_dir().join("window.txt")).unwrap_or_default();
    let v: Vec<f64> = s.split_whitespace().filter_map(|x| x.parse().ok()).collect();
    match v.as_slice() {
        [w, h, m] if *w >= 400.0 && *h >= 300.0 => (LogicalSize::new(*w, *h), *m == 1.0),
        _ => (LogicalSize::new(1200.0, 850.0), false),
    }
}

fn save_window_state(w: &Window) {
    let max = w.is_maximized();
    let (mut size, _) = load_window_state();
    if !max && !w.is_minimized() {
        size = w.inner_size().to_logical(w.scale_factor());
    }
    let _ = fs::create_dir_all(app_dir());
    let _ = fs::write(app_dir().join("window.txt"), format!("{} {} {}", size.width.round(), size.height.round(), max as u8));
}

fn is_dark() -> bool {
    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize")
        .and_then(|k| k.get_value::<u32, _>("AppsUseLightTheme"))
        .map(|v| v == 0)
        .unwrap_or(false)
}

// ---------- Association des fichiers .md ----------

fn open_command() -> io::Result<String> {
    Ok(format!("\"{}\" \"%1\"", std::env::current_exe()?.display()))
}

fn install() -> io::Result<()> {
    let exe = std::env::current_exe()?;
    let exe_name = exe.file_name().unwrap().to_string_lossy().into_owned();
    let open = open_command()?;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let set = |path: &str, name: &str, val: &str| -> io::Result<()> { hkcu.create_subkey(path)?.0.set_value(name, &val) };
    let cls = format!(r"Software\Classes\{PROG_ID}");
    let app = format!(r"Software\Classes\Applications\{exe_name}");
    set(&cls, "", "Document Markdown")?;
    set(&format!(r"{cls}\DefaultIcon"), "", &format!("\"{}\",0", exe.display()))?;
    set(&format!(r"{cls}\shell\open\command"), "", &open)?;
    set(&app, "FriendlyAppName", "MD Lite")?;
    set(&format!(r"{app}\shell\open\command"), "", &open)?;
    for e in EXTS {
        set(&format!(r"{app}\SupportedTypes"), e, "")?;
        set(r"Software\MDLite\Capabilities\FileAssociations", e, PROG_ID)?;
        hkcu.create_subkey(format!(r"Software\Classes\{e}\OpenWithProgids"))?
            .0
            .set_raw_value(PROG_ID, &RegValue { bytes: Cow::Borrowed(&[]), vtype: REG_NONE })?;
    }
    set(r"Software\MDLite\Capabilities", "ApplicationName", "MD Lite")?;
    set(r"Software\MDLite\Capabilities", "ApplicationDescription", "Lecteur et éditeur Markdown léger")?;
    set(r"Software\RegisteredApplications", "MD Lite", r"Software\MDLite\Capabilities")?;
    notify_assoc();
    Ok(())
}

fn uninstall() -> io::Result<()> {
    let exe = std::env::current_exe()?;
    let exe_name = exe.file_name().unwrap().to_string_lossy().into_owned();
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let _ = hkcu.delete_subkey_all(format!(r"Software\Classes\{PROG_ID}"));
    let _ = hkcu.delete_subkey_all(format!(r"Software\Classes\Applications\{exe_name}"));
    let _ = hkcu.delete_subkey_all(r"Software\MDLite");
    for e in EXTS {
        if let Ok(k) = hkcu.open_subkey_with_flags(format!(r"Software\Classes\{e}\OpenWithProgids"), KEY_SET_VALUE) {
            let _ = k.delete_value(PROG_ID);
        }
    }
    if let Ok(k) = hkcu.open_subkey_with_flags(r"Software\RegisteredApplications", KEY_SET_VALUE) {
        let _ = k.delete_value("MD Lite");
    }
    notify_assoc();
    Ok(())
}

// Si le dossier de MD Lite a été déplacé, l'association est remise à jour toute seule.
fn heal_registration() {
    let key = RegKey::predef(HKEY_CURRENT_USER).open_subkey(format!(r"Software\Classes\{PROG_ID}\shell\open\command"));
    if let (Ok(k), Ok(cmd)) = (key, open_command()) {
        if k.get_value::<String, _>("").map(|v| v != cmd).unwrap_or(true) {
            let _ = install();
        }
    }
}

fn is_default() -> bool {
    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\.md\UserChoice")
        .and_then(|k| k.get_value::<String, _>("ProgId"))
        .map(|p| p == PROG_ID || p.to_ascii_lowercase().contains("md-lite.exe"))
        .unwrap_or(false)
}

// Windows interdit à une application de se déclarer seule par défaut : on ouvre la fenêtre « Ouvrir avec » du système.
fn open_with_dialog(hwnd: isize) {
    let dir = app_dir();
    let _ = fs::create_dir_all(&dir);
    let sample = dir.join("exemple.md");
    let _ = fs::write(&sample, "# MD Lite\n");
    let file = wide(&sample.to_string_lossy());
    let info = OPENASINFO {
        pcszFile: file.as_ptr(),
        pcszClass: std::ptr::null(),
        oaifInFlags: OAIF_ALLOW_REGISTRATION | OAIF_REGISTER_EXT,
    };
    unsafe {
        SHOpenWithDialog(hwnd as _, &info);
    }
}

fn notify_assoc() {
    unsafe { SHChangeNotify(SHCNE_ASSOCCHANGED as _, SHCNF_IDLIST, std::ptr::null(), std::ptr::null()) }
}

fn report(r: io::Result<()>, quiet: bool, ok: &str) {
    if quiet && r.is_ok() {
        return;
    }
    let (level, msg) = match r {
        Ok(()) => (MessageLevel::Info, ok.to_string()),
        Err(e) => (MessageLevel::Error, format!("Échec : {e}")),
    };
    MessageDialog::new().set_title("MD Lite").set_level(level).set_description(msg).show();
}

// ---------- Utilitaires ----------

fn make_nonce() -> String {
    let mut h = std::collections::hash_map::RandomState::new().build_hasher();
    h.write_u128(SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0));
    format!("{:016x}", h.finish())
}

fn query(q: Option<&str>, key: &str) -> Option<String> {
    q?.split('&').find_map(|kv| {
        let (k, v) = kv.split_once('=').unwrap_or((kv, ""));
        (k == key).then(|| pct_decode(v))
    })
}

fn pct_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 3 <= b.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn pct_encode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

fn js_string(s: &str) -> String {
    let mut o = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '<' => o.push_str("\\u003c"),
            '\u{2028}' => o.push_str("\\u2028"),
            '\u{2029}' => o.push_str("\\u2029"),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}
