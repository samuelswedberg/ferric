#[cfg(any(windows, test))]
use std::path::{Path, PathBuf};

#[cfg(windows)]
const SHORTCUT_FILE_NAME: &str = "Ferric.lnk";
#[cfg(windows)]
const SHORTCUT_DESCRIPTION: &str = "A tiny, album-art-themed miniplayer for Spotify";

#[cfg(windows)]
#[derive(Debug, PartialEq, Eq)]
enum ShortcutAction {
    AlreadyRegistered(PathBuf),
    CreatedOrUpdated(PathBuf),
}

/// Ensures a per-user Start Menu shortcut exists for release builds on Windows
/// so Ferric is discoverable from Windows Search even when run outside an installer.
pub fn setup() {
    #[cfg(windows)]
    {
        if !should_auto_register(cfg!(debug_assertions)) {
            return;
        }
        if let Err(error) = std::thread::Builder::new()
            .name("ferric-shortcut".to_owned())
            .spawn(|| match ensure_current_exe_shortcut() {
                Ok(ShortcutAction::AlreadyRegistered(path)) => {
                    tracing::debug!(path = %path.display(), "Start Menu shortcut already registered");
                }
                Ok(ShortcutAction::CreatedOrUpdated(path)) => {
                    tracing::info!(path = %path.display(), "registered Start Menu shortcut");
                }
                Err(error) => {
                    tracing::warn!(%error, "failed to register Start Menu shortcut");
                }
            })
        {
            tracing::warn!(%error, "failed to spawn Start Menu shortcut registration thread");
        }
    }
}

#[cfg(any(windows, test))]
fn should_auto_register(debug_assertions: bool) -> bool {
    !debug_assertions
}

#[cfg(any(windows, test))]
fn candidate_shortcut_paths(user_programs: &Path, common_programs: Option<&Path>) -> Vec<PathBuf> {
    let mut paths = vec![
        user_programs.join("Ferric.lnk"),
        user_programs.join("Ferric").join("Ferric.lnk"),
        user_programs.join("Scoop Apps").join("Ferric.lnk"),
    ];
    if let Some(common) = common_programs {
        paths.push(common.join("Ferric.lnk"));
        paths.push(common.join("Ferric").join("Ferric.lnk"));
    }
    paths
}

#[cfg(any(windows, test))]
fn strip_verbatim_prefix(path: &Path) -> PathBuf {
    let raw = path.to_string_lossy();
    if let Some(unc) = raw.strip_prefix(r"\\?\UNC\") {
        PathBuf::from(format!(r"\\{unc}"))
    } else if let Some(stripped) = raw.strip_prefix(r"\\?\") {
        PathBuf::from(stripped)
    } else {
        path.to_path_buf()
    }
}

#[cfg(any(windows, test))]
fn normalize_path_for_shortcut(path: &Path) -> PathBuf {
    match std::fs::canonicalize(path) {
        Ok(canonical) => strip_verbatim_prefix(&canonical),
        Err(_) => strip_verbatim_prefix(path),
    }
}

#[cfg(any(windows, test))]
fn paths_match(left: &Path, right: &Path) -> bool {
    let left_norm = normalize_path_for_shortcut(left)
        .to_string_lossy()
        .replace('/', "\\");
    let right_norm = normalize_path_for_shortcut(right)
        .to_string_lossy()
        .replace('/', "\\");
    left_norm.eq_ignore_ascii_case(&right_norm)
}

#[cfg(windows)]
fn ensure_current_exe_shortcut() -> Result<ShortcutAction, String> {
    let exe_path = std::env::current_exe()
        .map_err(|error| format!("could not resolve current executable path: {error}"))?;
    let user_programs = known_folder_path(&windows::Win32::UI::Shell::FOLDERID_Programs)?;
    let common_programs =
        known_folder_path(&windows::Win32::UI::Shell::FOLDERID_CommonPrograms).ok();
    ensure_shortcut_in(&exe_path, &user_programs, common_programs.as_deref())
}

#[cfg(windows)]
fn ensure_shortcut_in(
    exe_path: &Path,
    user_programs: &Path,
    common_programs: Option<&Path>,
) -> Result<ShortcutAction, String> {
    let target_exe = normalize_path_for_shortcut(exe_path);
    let _com = ComApartment::initialize()?;

    for candidate in candidate_shortcut_paths(user_programs, common_programs) {
        if candidate.is_file() {
            if let Ok(existing_target) = read_shortcut_target(&candidate) {
                if paths_match(&existing_target, &target_exe) {
                    return Ok(ShortcutAction::AlreadyRegistered(candidate));
                }
            }
        }
    }

    std::fs::create_dir_all(user_programs)
        .map_err(|error| format!("could not create Start Menu folder: {error}"))?;
    let shortcut_path = user_programs.join(SHORTCUT_FILE_NAME);
    create_shortcut(&target_exe, &shortcut_path)?;
    Ok(ShortcutAction::CreatedOrUpdated(shortcut_path))
}

#[cfg(windows)]
fn known_folder_path(folder_id: &windows::core::GUID) -> Result<PathBuf, String> {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;

    use windows::Win32::{
        System::Com::CoTaskMemFree,
        UI::Shell::{SHGetKnownFolderPath, KF_FLAG_DEFAULT},
    };

    // SAFETY: `folder_id` points to a valid KNOWNFOLDERID GUID and `SHGetKnownFolderPath`
    // returns a CoTaskMem-allocated UTF-16 string that we free unconditionally via `CoTaskMemFree`.
    let raw = unsafe { SHGetKnownFolderPath(folder_id, KF_FLAG_DEFAULT, None) }
        .map_err(|error| format!("SHGetKnownFolderPath failed: {error}"))?;
    let path = {
        // SAFETY: `raw` is a valid null-terminated wide string returned by `SHGetKnownFolderPath`.
        let wide = unsafe { raw.as_wide() };
        PathBuf::from(OsString::from_wide(wide))
    };
    // SAFETY: `raw` was allocated by `SHGetKnownFolderPath` using the COM task allocator.
    unsafe { CoTaskMemFree(Some(raw.0.cast())) };
    Ok(path)
}

#[cfg(windows)]
fn read_shortcut_target(shortcut_path: &Path) -> Result<PathBuf, String> {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;

    use windows::{
        core::{Interface, HSTRING},
        Win32::{
            System::Com::{CoCreateInstance, IPersistFile, CLSCTX_INPROC_SERVER, STGM_READ},
            UI::Shell::{IShellLinkW, ShellLink, SLGP_RAWPATH},
        },
    };

    let link_str = HSTRING::from(shortcut_path.as_os_str());
    let mut target_buf = vec![0u16; 32_768];

    // SAFETY: COM is initialized on this thread by `ComApartment`, and all pointers/buffers
    // remain valid for the duration of each call.
    unsafe {
        let shell_link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)
            .map_err(|error| format!("CoCreateInstance(ShellLink) failed: {error}"))?;
        let persist: IPersistFile = shell_link
            .cast()
            .map_err(|error| format!("QueryInterface(IPersistFile) failed: {error}"))?;
        persist
            .Load(&link_str, STGM_READ)
            .map_err(|error| format!("IPersistFile::Load failed: {error}"))?;
        shell_link
            .GetPath(&mut target_buf, std::ptr::null_mut(), SLGP_RAWPATH.0 as u32)
            .map_err(|error| format!("IShellLinkW::GetPath failed: {error}"))?;
    }

    let len = target_buf
        .iter()
        .position(|&unit| unit == 0)
        .unwrap_or(target_buf.len());
    Ok(PathBuf::from(OsString::from_wide(&target_buf[..len])))
}

#[cfg(windows)]
fn create_shortcut(target_exe: &Path, shortcut_path: &Path) -> Result<(), String> {
    use windows::{
        core::{Interface, HSTRING},
        Win32::{
            System::Com::{CoCreateInstance, IPersistFile, CLSCTX_INPROC_SERVER},
            UI::Shell::{IShellLinkW, ShellLink},
        },
    };

    let target_str = HSTRING::from(target_exe.as_os_str());
    let working_dir = target_exe
        .parent()
        .map(|dir| HSTRING::from(dir.as_os_str()));
    let description = HSTRING::from(SHORTCUT_DESCRIPTION);
    let link_str = HSTRING::from(shortcut_path.as_os_str());

    // SAFETY: COM is initialized on this thread by `ComApartment`, and every `HSTRING` lives
    // across its corresponding COM method call.
    unsafe {
        let shell_link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)
            .map_err(|error| format!("CoCreateInstance(ShellLink) failed: {error}"))?;
        shell_link
            .SetPath(&target_str)
            .map_err(|error| format!("IShellLinkW::SetPath failed: {error}"))?;
        if let Some(dir) = working_dir.as_ref() {
            shell_link
                .SetWorkingDirectory(dir)
                .map_err(|error| format!("IShellLinkW::SetWorkingDirectory failed: {error}"))?;
        }
        shell_link
            .SetDescription(&description)
            .map_err(|error| format!("IShellLinkW::SetDescription failed: {error}"))?;
        shell_link
            .SetIconLocation(&target_str, 0)
            .map_err(|error| format!("IShellLinkW::SetIconLocation failed: {error}"))?;

        let persist: IPersistFile = shell_link
            .cast()
            .map_err(|error| format!("QueryInterface(IPersistFile) failed: {error}"))?;
        persist
            .Save(&link_str, true)
            .map_err(|error| format!("IPersistFile::Save failed: {error}"))?;
    }

    Ok(())
}

#[cfg(windows)]
struct ComApartment {
    should_uninitialize: bool,
}

#[cfg(windows)]
impl ComApartment {
    fn initialize() -> Result<Self, String> {
        use windows::Win32::{
            Foundation::RPC_E_CHANGED_MODE,
            System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED},
        };

        // SAFETY: standard per-thread COM initialization paired with `CoUninitialize` on Drop
        // only when `CoInitializeEx` succeeded (S_OK or S_FALSE).
        let hr = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
        if hr.is_ok() {
            Ok(Self {
                should_uninitialize: true,
            })
        } else if hr == RPC_E_CHANGED_MODE {
            Ok(Self {
                should_uninitialize: false,
            })
        } else {
            Err(format!("CoInitializeEx failed: {hr:?}"))
        }
    }
}

#[cfg(windows)]
impl Drop for ComApartment {
    fn drop(&mut self) {
        if self.should_uninitialize {
            // SAFETY: balanced with a successful `CoInitializeEx` on this thread.
            unsafe { windows::Win32::System::Com::CoUninitialize() };
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{
        candidate_shortcut_paths, normalize_path_for_shortcut, paths_match, should_auto_register,
    };

    #[test]
    fn auto_registration_skips_debug_builds_and_runs_in_release() {
        assert!(!should_auto_register(true));
        assert!(should_auto_register(false));
    }

    #[test]
    fn candidate_shortcut_paths_check_user_scoop_and_common_locations() {
        let user =
            Path::new(r"C:\Users\Test\AppData\Roaming\Microsoft\Windows\Start Menu\Programs");
        let common = Path::new(r"C:\ProgramData\Microsoft\Windows\Start Menu\Programs");

        let candidates = candidate_shortcut_paths(user, Some(common));
        assert_eq!(
            candidates,
            vec![
                user.join("Ferric.lnk"),
                user.join("Ferric").join("Ferric.lnk"),
                user.join("Scoop Apps").join("Ferric.lnk"),
                common.join("Ferric.lnk"),
                common.join("Ferric").join("Ferric.lnk"),
            ]
        );
    }

    #[test]
    fn path_matching_ignores_case_slashes_and_verbatim_prefix() {
        assert_eq!(
            normalize_path_for_shortcut(Path::new(r"\\?\C:\Tools\Ferric\Ferric.exe")),
            PathBuf::from(r"C:\Tools\Ferric\Ferric.exe")
        );
        assert_eq!(
            normalize_path_for_shortcut(Path::new(r"\\?\UNC\server\share\Ferric.exe")),
            PathBuf::from(r"\\server\share\Ferric.exe")
        );
        assert!(paths_match(
            Path::new(r"\\?\C:\Tools\Ferric\Ferric.exe"),
            Path::new(r"c:/tools/ferric/ferric.exe")
        ));
        assert!(!paths_match(
            Path::new(r"C:\Tools\Ferric\Ferric.exe"),
            Path::new(r"C:\Other\Ferric.exe")
        ));
    }

    #[cfg(windows)]
    mod windows_tests {
        use std::{
            fs,
            path::PathBuf,
            sync::atomic::{AtomicU64, Ordering},
        };

        use super::super::{
            create_shortcut, ensure_shortcut_in, paths_match, read_shortcut_target, ComApartment,
            ShortcutAction,
        };

        static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(1);

        struct TempDir {
            path: PathBuf,
        }

        impl TempDir {
            fn new(prefix: &str) -> Self {
                let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
                let path = std::env::temp_dir().join(format!(
                    "ferric-shortcut-test-{prefix}-{}-{id}",
                    std::process::id()
                ));
                fs::create_dir_all(&path).expect("create temp test dir");
                Self { path }
            }
        }

        impl Drop for TempDir {
            fn drop(&mut self) {
                let _ = fs::remove_dir_all(&self.path);
            }
        }

        #[test]
        fn creates_reuses_and_updates_user_start_menu_shortcut() {
            let root = TempDir::new("lifecycle");
            let user_programs = root.path.join("UserPrograms");
            let exe_v1 = root.path.join("v1").join("Ferric.exe");
            let exe_v2 = root.path.join("v2").join("Ferric.exe");
            fs::create_dir_all(exe_v1.parent().expect("v1 dir")).expect("create v1 dir");
            fs::create_dir_all(exe_v2.parent().expect("v2 dir")).expect("create v2 dir");
            fs::write(&exe_v1, b"").expect("create fake exe v1");
            fs::write(&exe_v2, b"").expect("create fake exe v2");

            let expected_lnk = user_programs.join("Ferric.lnk");

            let first = ensure_shortcut_in(&exe_v1, &user_programs, None)
                .expect("initial shortcut creation");
            assert_eq!(
                first,
                ShortcutAction::CreatedOrUpdated(expected_lnk.clone())
            );
            assert!(expected_lnk.is_file());

            {
                let _com = ComApartment::initialize().expect("com init");
                let target = read_shortcut_target(&expected_lnk).expect("read shortcut target");
                assert!(
                    paths_match(&target, &exe_v1),
                    "expected {} to match {}",
                    target.display(),
                    exe_v1.display()
                );
                let dotdot_v1 = root
                    .path
                    .join("v1")
                    .join("..")
                    .join("v1")
                    .join("Ferric.exe");
                assert!(paths_match(&target, &dotdot_v1));
            }

            let second = ensure_shortcut_in(&exe_v1, &user_programs, None)
                .expect("idempotent shortcut check");
            assert_eq!(
                second,
                ShortcutAction::AlreadyRegistered(expected_lnk.clone())
            );

            let updated =
                ensure_shortcut_in(&exe_v2, &user_programs, None).expect("stale shortcut update");
            assert_eq!(
                updated,
                ShortcutAction::CreatedOrUpdated(expected_lnk.clone())
            );

            {
                let _com = ComApartment::initialize().expect("com init");
                let target = read_shortcut_target(&expected_lnk).expect("read updated target");
                assert!(paths_match(&target, &exe_v2));
            }
        }

        #[test]
        fn avoids_duplicate_when_common_installer_shortcut_already_matches() {
            let root = TempDir::new("common");
            let user_programs = root.path.join("UserPrograms");
            let common_programs = root.path.join("CommonPrograms");
            let installer_dir = common_programs.join("Ferric");
            let exe = root.path.join("Ferric.exe");
            fs::create_dir_all(&installer_dir).expect("create installer start menu dir");
            fs::write(&exe, b"").expect("create fake exe");

            let common_lnk = installer_dir.join("Ferric.lnk");
            {
                let _com = ComApartment::initialize().expect("com init");
                create_shortcut(&exe, &common_lnk).expect("create common shortcut");
            }

            let action = ensure_shortcut_in(&exe, &user_programs, Some(&common_programs))
                .expect("shortcut check with common installer shortcut");
            assert_eq!(action, ShortcutAction::AlreadyRegistered(common_lnk));
            assert!(!user_programs.join("Ferric.lnk").exists());
        }
    }
}
