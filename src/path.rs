use std::{borrow::Cow, path::Path};

// Looks like std::path::Path is system specific. On macOS and Linux, using '\' instead of '/' in
// the path causes file_name and file_stem to return the entire string.
pub trait PathPal {
    fn file_stem_pal<'a>(&'a self) -> Option<Cow<'a, str>>;
    fn file_name_pal<'a>(&'a self) -> Option<Cow<'a, str>>;
}

#[cfg(not(target_family = "unix"))]
impl PathPal for Path {
    fn file_stem_pal<'a>(&'a self) -> Option<Cow<'a, str>> {
        Some(Cow::Borrowed(self.file_stem()?.to_str()?))
    }

    fn file_name_pal<'a>(&'a self) -> Option<Cow<'a, str>> {
        Some(Cow::Borrowed(self.file_name()?.to_str()?))
    }
}

#[cfg(target_family = "unix")]
impl PathPal for Path {
    fn file_stem_pal<'a>(&'a self) -> Option<Cow<'a, str>> {
        let base_str = self.to_str()?;
        let new_string = base_str.replace('\\', "/");
        let new_path = PathBuf::from(new_string);
        let stem = new_path.file_stem()?.to_str()?.to_owned();
        Some(Cow::Owned(stem))
    }

    fn file_name_pal<'a>(&'a self) -> Option<Cow<'a, str>> {
        let base_str = self.to_str()?;
        let new_string = base_str.replace('\\', "/");
        let new_path = PathBuf::from(new_string);
        let name = new_path.file_name()?.to_str()?.to_owned();
        Some(Cow::Owned(name))
    }
}
