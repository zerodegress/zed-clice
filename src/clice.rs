use std::{
    fs,
    path::{self, PathBuf},
};

use zed_extension_api::{self as zed, LanguageServerId, Result};

struct CliceExtension {
    cached_lsp_binary_path: Option<String>,
    resource_dir: Option<String>,
}

impl CliceExtension {
    fn language_server_binary_path(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<String> {
        if let Some(cached) = self.cached_lsp_binary_path.to_owned() {
            return Ok(cached);
        }

        if let Some(path) = worktree.which("clice") {
            return Ok(path.to_owned());
        }

        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::CheckingForUpdate,
        );
        let release = zed::latest_github_release(
            "zerodegress/clice-build",
            zed::GithubReleaseOptions {
                require_assets: true,
                pre_release: true,
            },
        )?;

        let (platform, arch) = zed::current_platform();
        let asset_name = format!(
            "clice-{arch}-{os}.tar.gz",
            arch = match arch {
                zed::Architecture::X8664 => "x86_64",
                zed::Architecture::Aarch64 => "aarch64",
                zed::Architecture::X86 => {
                    return Err(format!("unsupported architecture: {arch:?}"));
                }
            },
            os = match platform {
                zed::Os::Windows => "windows-msvc",
                zed::Os::Linux => "linux-gnu",
                zed::Os::Mac => "macos-darwin",
            }
        );

        let asset = release
            .assets
            .iter()
            .find(|asset| asset.name == asset_name)
            .ok_or_else(|| format!("no asset found matching {:?}", asset_name))?;

        let version_dir = PathBuf::from(format!("clice-{}", release.version));
        let _ = fs::create_dir_all(&version_dir);
        let package_dir = version_dir.join("clice");
        let binary_file = package_dir.join("clice");
        let resource_dir = package_dir.join("lib/clang/20");

        if !fs::metadata(&binary_file).is_ok_and(|stat| stat.is_file()) {
            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::Downloading,
            );

            zed::download_file(
                &asset.download_url,
                &version_dir.to_string_lossy(),
                zed::DownloadedFileType::GzipTar,
            )
            .map_err(|e| format!("failed to download file: {e}"))?;
        }

        self.resource_dir = Some(
            path::absolute(resource_dir)
                .map_err(|err| format!("path cannot be resolved: {err}"))?
                .to_string_lossy()
                .to_string(),
        );
        self.cached_lsp_binary_path = Some(binary_file.to_string_lossy().to_string());

        Ok(binary_file.to_string_lossy().to_string())
    }

    fn resource_dir_path(&self) -> Result<Option<String>> {
        Ok(self.resource_dir.to_owned())
    }
}

impl zed::Extension for CliceExtension {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            cached_lsp_binary_path: Default::default(),
            resource_dir: Default::default(),
        }
    }

    fn language_server_command(
        &mut self,
        language_server_id: &zed_extension_api::LanguageServerId,
        worktree: &zed_extension_api::Worktree,
    ) -> zed_extension_api::Result<zed_extension_api::Command> {
        let cmd = zed::Command {
            command: { self.language_server_binary_path(language_server_id, worktree)? },
            args: self
                .resource_dir_path()?
                .map(|resource_dir| vec![format!("--resource-dir={resource_dir}")])
                .unwrap_or(Vec::new()),
            env: Default::default(),
        };
        Ok(cmd)
    }
}

zed::register_extension!(CliceExtension);
