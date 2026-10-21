// Partie propre à Linux : WebKitGTK, fichier .desktop + xdg-mime (association .md),
// instance unique par socket Unix, ouverture des liens avec xdg-open.

use std::fs;
use std::io::{self, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use rfd::{FileDialog, MessageDialog};
use tao::event_loop::EventLoopProxy;
use tao::platform::unix::WindowExtUnix;
use tao::window::{Icon, Window, WindowBuilder};
use wry::{WebView, WebViewBuilder, WebViewBuilderExtUnix};

use crate::{UserEvent, ICON_PNG};

pub const ORIGIN: &str = "mdlite://localhost";
pub const ENGINE_HELP: &str = "MD Lite a besoin de WebKitGTK 4.1 (version 2.40 ou plus récente). Sous Debian/Ubuntu : sudo apt install libwebkit2gtk-4.1-0";

const DESKTOP_FILE: &str = "md-lite.desktop";
const MIME_TYPES: [&str; 2] = ["text/markdown", "text/x-markdown"];

fn home() -> PathBuf {
    std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(std::env::temp_dir)
}

fn xdg(var: &str, fallback: &str) -> PathBuf {
    std::env::var_os(var).map(PathBuf::from).filter(|p| p.is_absolute()).unwrap_or_else(|| home().join(fallback))
}

pub fn app_dir() -> PathBuf {
    xdg("XDG_CONFIG_HOME", ".config").join("md-lite")
}

fn data_dir() -> PathBuf {
    xdg("XDG_DATA_HOME", ".local/share")
}

// ---------- Fenêtre ----------

fn icon() -> Option<Icon> {
    let mut reader = png::Decoder::new(io::Cursor::new(ICON_PNG)).read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buf).ok()?;
    if info.color_type != png::ColorType::Rgba || info.bit_depth != png::BitDepth::Eight {
        return None;
    }
    buf.truncate(info.buffer_size());
    Icon::from_rgba(buf, info.width, info.height).ok()
}

pub fn window_builder(b: WindowBuilder) -> WindowBuilder {
    b.with_window_icon(icon())
}

pub fn window_id(_w: &Window) -> isize {
    0
}

// Sous Wayland comme sous X11, la vue web est placée dans le conteneur GTK de la fenêtre.
pub fn build_webview(b: WebViewBuilder, w: &Window) -> wry::Result<WebView> {
    b.build_gtk(w.default_vbox().expect("conteneur GTK de la fenêtre"))
}

fn socket_path() -> PathBuf {
    match std::env::var_os("XDG_RUNTIME_DIR") {
        Some(d) => PathBuf::from(d).join("md-lite.sock"),
        None => std::env::temp_dir().join(format!("md-lite-{}.sock", std::env::var("USER").unwrap_or_default())),
    }
}

// La première fenêtre écoute sur un socket Unix : les lancements suivants lui confient leurs fichiers.
pub fn attach(_w: &Window, proxy: EventLoopProxy<UserEvent>) {
    let path = socket_path();
    if UnixStream::connect(&path).is_ok() {
        return; // une autre fenêtre écoute déjà
    }
    let _ = fs::remove_file(&path);
    let Ok(listener) = UnixListener::bind(&path) else { return };
    std::thread::spawn(move || {
        for mut stream in listener.incoming().flatten() {
            let mut s = String::new();
            if stream.read_to_string(&mut s).is_ok() {
                let paths = s.split('\n').filter(|l| !l.is_empty()).map(PathBuf::from).collect();
                let _ = proxy.send_event(UserEvent::Open(paths));
            }
        }
    });
}

pub fn detach() {
    let path = socket_path();
    // Ne supprime le socket que s'il est le nôtre (plus personne n'y répond après notre fermeture).
    if UnixStream::connect(&path).is_ok() {
        let _ = fs::remove_file(&path);
    }
}

pub fn hand_off(files: &[PathBuf]) -> bool {
    let Ok(mut stream) = UnixStream::connect(socket_path()) else { return false };
    let list: Vec<String> = files.iter().map(|p| p.to_string_lossy().into_owned()).collect();
    stream.write_all(list.join("\n").as_bytes()).is_ok()
}

pub fn is_dark() -> bool {
    let gsettings = |key: &str| {
        Command::new("gsettings")
            .args(["get", "org.gnome.desktop.interface", key])
            .stderr(Stdio::null())
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).to_lowercase())
            .unwrap_or_default()
    };
    if gsettings("color-scheme").contains("prefer-dark") {
        return true;
    }
    let theme = std::env::var("GTK_THEME").unwrap_or_default().to_lowercase() + &gsettings("gtk-theme");
    theme.contains("dark")
}

pub fn file_dialog(_win: isize) -> FileDialog {
    FileDialog::new()
}

pub fn message_dialog(_win: isize) -> MessageDialog {
    MessageDialog::new()
}

// ---------- Ouverture de liens et de fichiers ----------

pub fn shell_open(target: &str) {
    let _ = Command::new("xdg-open").arg(target).stdout(Stdio::null()).stderr(Stdio::null()).spawn();
}

pub fn show_in_folder(p: &Path) {
    if let Some(dir) = p.parent() {
        shell_open(&dir.to_string_lossy());
    }
}

pub fn is_executable(p: &Path) -> bool {
    fs::metadata(p).map(|m| m.permissions().mode() & 0o111 != 0).unwrap_or(false)
}

// ---------- Association des fichiers .md ----------

fn desktop_path() -> PathBuf {
    data_dir().join("applications").join(DESKTOP_FILE)
}

// Guillemets et échappements de la ligne Exec selon la spécification freedesktop.
fn exec_line() -> io::Result<String> {
    let exe = std::env::current_exe()?.to_string_lossy().into_owned();
    let quoted: String = exe
        .chars()
        .flat_map(|c| match c {
            '"' | '`' | '$' | '\\' => vec!['\\', c],
            c => vec![c],
        })
        .collect();
    Ok(format!("Exec=\"{quoted}\" %F"))
}

pub fn install() -> io::Result<()> {
    let icon_dir = data_dir().join("icons/hicolor/256x256/apps");
    fs::create_dir_all(&icon_dir)?;
    fs::write(icon_dir.join("md-lite.png"), ICON_PNG)?;
    let desktop = desktop_path();
    fs::create_dir_all(desktop.parent().unwrap())?;
    fs::write(
        &desktop,
        format!(
            "[Desktop Entry]\nType=Application\nName=MD Lite\nGenericName=Markdown Editor\nComment=Lightweight Markdown reader and editor\nComment[fr]=Lecteur et éditeur Markdown léger\n{}\nIcon=md-lite\nTerminal=false\nCategories=Office;TextEditor;Utility;\nMimeType={};\nStartupWMClass=md-lite\n",
            exec_line()?,
            MIME_TYPES.join(";")
        ),
    )?;
    let _ = Command::new("update-desktop-database").arg(desktop.parent().unwrap()).stderr(Stdio::null()).status();
    Ok(())
}

pub fn uninstall() -> io::Result<()> {
    let _ = fs::remove_file(desktop_path());
    let _ = fs::remove_file(data_dir().join("icons/hicolor/256x256/apps/md-lite.png"));
    Ok(())
}

// Si l'exécutable a été déplacé, le fichier .desktop est remis à jour tout seul.
pub fn heal_registration() {
    if let (Ok(content), Ok(exec)) = (fs::read_to_string(desktop_path()), exec_line()) {
        if !content.lines().any(|l| l == exec) {
            let _ = install();
        }
    }
}

pub fn is_default() -> bool {
    Command::new("xdg-mime")
        .args(["query", "default", MIME_TYPES[0]])
        .stderr(Stdio::null())
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == DESKTOP_FILE)
        .unwrap_or(false)
}

// Sous Linux, une application peut se déclarer par défaut directement.
pub fn make_default(_win: isize) -> io::Result<()> {
    install()?;
    for mime in MIME_TYPES {
        Command::new("xdg-mime").args(["default", DESKTOP_FILE, mime]).status()?;
    }
    Ok(())
}
