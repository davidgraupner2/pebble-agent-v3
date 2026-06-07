use std::path::Path;
use std::{env::current_exe, path::PathBuf};

#[derive(Debug, Clone)]
pub struct Folders {
    home_folder: PathBuf,
    supplementrary_files_folder: PathBuf,
    jobs_folder: PathBuf,
    deno_cache_folder: PathBuf,
    logs_folder: PathBuf,
    plugins_folder: PathBuf,
    discovery_folder: PathBuf,
}

impl Folders {
    #[cfg(target_os = "windows")]
    pub fn new(_app_name: &str) -> Self {
        let base = std::env::var_os("PROGRAMDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(r"C:\ProgramData"));

        let home_folder = current_exe().unwrap().parent().unwrap().to_path_buf();
        let supplementrary_files_folder = home_folder.join("supplementary_files");
        let logs_folder = home_folder.join("logs");
        let jobs_folder = home_folder.join("jobs");
        let deno_cache_folder = home_folder.join("jobs").join(".deno_cache");
        let plugins_folder = supplementrary_files_folder.join("plugins");
        let discovery_folder = base.join("Pebble").join("run");

        Self {
            home_folder,
            logs_folder,
            supplementrary_files_folder,
            jobs_folder,
            deno_cache_folder,
            discovery_folder,
            plugins_folder,
        }
    }
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    pub fn new(#[cfg_attr(debug_assertions, allow(unused_variables))] app_name: &str) -> Self {
        let home_folder = current_exe().unwrap().parent().unwrap().to_path_buf();

        let supplementrary_files_folder;
        let logs_folder;
        let jobs_folder;
        let discovery_folder: PathBuf;

        #[cfg(debug_assertions)]
        {
            supplementrary_files_folder = home_folder.join("supplementary_files");
            logs_folder = home_folder.join("logs");
            jobs_folder = home_folder.join("jobs");
            discovery_folder = home_folder.join("discovery");
        }

        #[cfg(not(debug_assertions))]
        {
            supplementrary_files_folder = home_folder.join("supplementary_files");
            logs_folder = PathBuf::from("/").join("var").join("log").join(app_name);
            jobs_folder = PathBuf::from("/").join("opt").join(app_name).join("jobs");

            #[cfg(target_os = "linux")]
            {
                discovery_folder = PathBuf::from("/").join("run").join("pebble");
            }

            #[cfg(target_os = "macos")]
            {
                discovery_folder = PathBuf::from("/").join("var").join("run").join("pebble");
            }
        }

        let plugins_folder = supplementrary_files_folder.join("plugins");
        let deno_cache_folder = jobs_folder.join(".deno_cache");

        Self {
            home_folder,
            logs_folder,
            supplementrary_files_folder,
            jobs_folder,
            plugins_folder,
            deno_cache_folder,
            discovery_folder,
        }
    }

    /// Construct folders using an explicit base path. This is useful for tests
    /// where you want to control where folders are created.
    pub fn new_with_base(_app_name: &str, base: &Path) -> Self {
        let home_folder = base.to_path_buf();

        #[cfg(debug_assertions)]
        let supplementrary_files_folder = home_folder.join("supplementary_files");

        #[cfg(not(debug_assertions))]
        let supplementrary_files_folder = home_folder.join("supplementary_files");

        #[cfg(debug_assertions)]
        let logs_folder = home_folder.join("logs");

        #[cfg(not(debug_assertions))]
        let logs_folder = home_folder.join("logs");

        #[cfg(debug_assertions)]
        let discovery_folder = home_folder.join("discovery");

        #[cfg(not(debug_assertions))]
        let discovery_folder = home_folder.join("discovery");

        #[cfg(debug_assertions)]
        let jobs_folder = home_folder.join("jobs");

        #[cfg(not(debug_assertions))]
        let jobs_folder = home_folder.join("jobs");

        let plugins_folder = supplementrary_files_folder.join("plugins");
        let deno_cache_folder = jobs_folder.join(".deno_cache");

        Self {
            home_folder,
            logs_folder,
            supplementrary_files_folder,
            jobs_folder,
            plugins_folder,
            deno_cache_folder,
            discovery_folder,
        }
    }

    pub fn ensure_exists(self) -> Self {
        if !self.home_folder.exists() {
            std::fs::create_dir_all(&self.home_folder).unwrap();
        }

        if !self.supplementrary_files_folder.exists() {
            std::fs::create_dir_all(&self.supplementrary_files_folder).unwrap();
        }

        if !self.logs_folder.exists() {
            std::fs::create_dir_all(&self.logs_folder).unwrap();
        }

        if !self.jobs_folder.exists() {
            std::fs::create_dir_all(&self.jobs_folder).unwrap();
        }

        if !self.deno_cache_folder.exists() {
            std::fs::create_dir_all(&self.deno_cache_folder).unwrap();
        }

        if !self.plugins_folder.exists() {
            std::fs::create_dir_all(&self.plugins_folder).unwrap();
        }

        if !self.discovery_folder.exists() {
            std::fs::create_dir_all(&self.discovery_folder).unwrap();
        }

        self
    }

    // Accessor methods
    pub fn home(&self) -> &PathBuf {
        &self.home_folder
    }
    pub fn supplementary_files(&self) -> &PathBuf {
        &self.supplementrary_files_folder
    }
    pub fn logs(&self) -> &PathBuf {
        &self.logs_folder
    }
    pub fn jobs(&self) -> &PathBuf {
        &self.jobs_folder
    }
    pub fn deno_cache(&self) -> &PathBuf {
        &self.deno_cache_folder
    }
    pub fn plugins(&self) -> &PathBuf {
        &self.plugins_folder
    }

    pub fn discovery(&self) -> &PathBuf {
        &self.discovery_folder
    }
}
