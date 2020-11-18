use log::info;
use privdrop::PrivDrop;
use std::{
  ffi::OsString,
  path::PathBuf,
};
use structopt::StructOpt;
use super::errx;

#[derive(Debug, StructOpt)]
pub(crate) struct PrivDropConfig {
    /// Run as this user and their primary group
    #[structopt(short = "u", long = "user", parse(from_os_str))]
    user: Option<OsString>,
    /// Run as this group
    #[structopt(short = "g", long = "group", parse(from_os_str))]
    group: Option<OsString>,
    /// Chroot to this directory
    #[structopt(long = "chroot", parse(from_os_str))]
    chroot: Option<PathBuf>,
}

impl PrivDropConfig {
    pub(crate) fn drop(
        &self,
    ) {
        if self.user.is_some()
        || self.group.is_some()
        || self.chroot.is_some()
        {
            let mut pd = PrivDrop::default();
            if let Some(path) = &self.chroot {
                info!("privdrop, chroot: {}", path.display());
                pd = pd.chroot(path);
            }

            if let Some(user) = &self.user {
                info!("privdrop, user: {}", user.to_string_lossy());
                pd = pd.user(user);
            }

            if let Some(group) = &self.group {
                info!("privdrop, group: {}", group.to_string_lossy());
                pd = pd.group(group);
            }

            pd.apply()
                .unwrap_or_else(|err| errx(exitcode::OSERR, format!("privdrop, error: {}", err)));

            info!("privdrop, enabled: true");
        } else {
            info!("privdrop, enabled: false");
        }
    }
}
