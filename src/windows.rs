// Partie propre à Windows : WebView2, registre (association .md), instance unique par WM_COPYDATA.

use std::borrow::Cow;
use std::fs;
use std::io;
use std::num::NonZeroIsize;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};

use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawWindowHandle, Win32WindowHandle, WindowHandle,
};
use rfd::{FileDialog, MessageDialog};
use tao::event_loop::EventLoopProxy;
use tao::platform::windows::{IconExtWindows, WindowBuilderExtWindows, WindowExtWindows};
use tao::window::{Icon, Window, WindowBuilder};
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
use wry::{WebView, WebViewBuilder, WebViewBuilderExtWindows};

use crate::UserEvent;

pub const ORIGIN: &str = "https://mdlite.localhost";
pub const ENGINE_HELP: &str = "MD Lite a besoin du runtime Microsoft WebView2 (fourni avec Windows 10/11 à jour) : https://developer.microsoft.com/microsoft-edge/webview2/";

const PROG_ID: &str = "MDLite.Markdown";
const EXTS: [&str; 2] = [".md", ".markdown"];
// Marque posée sur nos fenêtres pour qu'un second lancement les retrouve.
const WINDOW_PROP: &str = "MDLite.Window";
const COPY_ID: usize = 0x4D_444C;

static PROXY: OnceLock<Mutex<EventLoopProxy<UserEvent>>> = OnceLock::new();

pub fn app_dir() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    base.join("MDLite")
}

// ---------- Fenêtre ----------

pub fn window_builder(b: WindowBuilder) -> WindowBuilder {
    b.with_window_icon(Icon::from_resource(1, None).ok()).with_taskbar_icon(Icon::from_resource(1, None).ok())
}

pub fn window_id(w: &Window) -> isize {
    w.hwnd()
}

pub fn build_webview(b: WebViewBuilder, w: &Window) -> wry::Result<WebView> {
    b.with_https_scheme(true).with_browser_accelerator_keys(false).build(w)
}

pub fn attach(w: &Window, proxy: EventLoopProxy<UserEvent>) {
    let _ = PROXY.set(Mutex::new(proxy));
    let hwnd = w.hwnd() as HWND;
    unsafe {
        let prop = wide(WINDOW_PROP);
        SetPropW(hwnd, prop.as_ptr(), 1 as _);
        SetWindowSubclass(hwnd, Some(subclass), 1, 0);
    }
}

pub fn detach() {}

pub fn is_dark() -> bool {
    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize")
        .and_then(|k| k.get_value::<u32, _>("AppsUseLightTheme"))
        .map(|v| v == 0)
        .unwrap_or(false)
}

// Boîtes de dialogue rattachées à la fenêtre (modales).
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

pub fn file_dialog(win: isize) -> FileDialog {
    FileDialog::new().set_parent(&Parent(win))
}

pub fn message_dialog(win: isize) -> MessageDialog {
    MessageDialog::new().set_parent(&Parent(win))
}

// ---------- Ouverture de liens et de fichiers ----------

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

pub fn shell_open(target: &str) {
    let (op, file) = (wide("open"), wide(target));
    unsafe {
        ShellExecuteW(std::ptr::null_mut(), op.as_ptr(), file.as_ptr(), std::ptr::null(), std::ptr::null(), SW_SHOWNORMAL);
    }
}

pub fn show_in_folder(p: &Path) {
    let _ = Command::new("explorer").arg(format!("/select,{}", p.display())).spawn();
}

pub fn is_executable(_p: &Path) -> bool {
    false
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
pub fn hand_off(files: &[PathBuf]) -> bool {
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

// ---------- Association des fichiers .md ----------

fn open_command() -> io::Result<String> {
    Ok(format!("\"{}\" \"%1\"", std::env::current_exe()?.display()))
}

pub fn install() -> io::Result<()> {
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

pub fn uninstall() -> io::Result<()> {
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
pub fn heal_registration() {
    let key = RegKey::predef(HKEY_CURRENT_USER).open_subkey(format!(r"Software\Classes\{PROG_ID}\shell\open\command"));
    if let (Ok(k), Ok(cmd)) = (key, open_command()) {
        if k.get_value::<String, _>("").map(|v| v != cmd).unwrap_or(true) {
            let _ = install();
        }
    }
}

pub fn is_default() -> bool {
    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\.md\UserChoice")
        .and_then(|k| k.get_value::<String, _>("ProgId"))
        .map(|p| p == PROG_ID || p.to_ascii_lowercase().contains("md-lite.exe"))
        .unwrap_or(false)
}

// Windows interdit à une application de se déclarer seule par défaut : on ouvre la fenêtre « Ouvrir avec » du système.
pub fn make_default(win: isize) -> io::Result<()> {
    install()?;
    let dir = app_dir();
    fs::create_dir_all(&dir)?;
    let sample = dir.join("exemple.md");
    fs::write(&sample, "# MD Lite\n")?;
    let file = wide(&sample.to_string_lossy());
    let info = OPENASINFO {
        pcszFile: file.as_ptr(),
        pcszClass: std::ptr::null(),
        oaifInFlags: OAIF_ALLOW_REGISTRATION | OAIF_REGISTER_EXT,
    };
    unsafe {
        SHOpenWithDialog(win as _, &info);
    }
    Ok(())
}

fn notify_assoc() {
    unsafe { SHChangeNotify(SHCNE_ASSOCCHANGED as _, SHCNF_IDLIST, std::ptr::null(), std::ptr::null()) }
}
