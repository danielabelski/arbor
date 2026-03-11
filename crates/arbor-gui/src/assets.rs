use super::*;

pub(crate) fn find_assets_root_dir() -> Option<PathBuf> {
    if let Ok(exe) = env::current_exe() {
        let exe_dir = exe.parent()?;

        let macos_bundle = exe_dir.parent().map(|path| path.join("Resources"));
        if let Some(dir) = macos_bundle
            && dir.is_dir()
        {
            return Some(dir);
        }

        let share_dir = exe_dir
            .parent()
            .map(|path| path.join("share").join("arbor"));
        if let Some(dir) = share_dir
            && dir.is_dir()
        {
            return Some(dir);
        }
    }

    let dev_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    if dev_dir.is_dir() {
        return Some(dev_dir);
    }

    None
}

pub(crate) fn find_asset_dir(relative_subdir: &str) -> Option<PathBuf> {
    let dir = find_assets_root_dir()?.join(relative_subdir);
    dir.is_dir().then_some(dir)
}

pub(crate) fn find_top_bar_icons_dir() -> Option<PathBuf> {
    static TOP_BAR_ICONS_DIR: OnceLock<Option<PathBuf>> = OnceLock::new();

    TOP_BAR_ICONS_DIR
        .get_or_init(|| find_asset_dir("icons/top-bar"))
        .clone()
}
