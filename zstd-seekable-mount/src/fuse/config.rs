use fuser::{Config, MountOption, SessionACL};

pub fn default_config() -> Config {
    let mut config = Config::default();

    // config.mount_options.push(MountOption::AutoUnmount);
    config.mount_options.push(MountOption::RO);
    config
        .mount_options
        .push(MountOption::FSName("fuse-zstd-seekable-mount".into()));

    config.acl = SessionACL::All;
    config.n_threads = Some(num_cpus::get());
    config.clone_fd = true;

    config
}
