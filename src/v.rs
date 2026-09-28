use std::fs;
use zed::{CodeLabel, CodeLabelSpan, LanguageServerId};
use zed_extension_api::{self as zed, CodeLabelSpanLiteral, Result};

struct VExtension {
	current_version: String,
	cached_binary_path: Option<String>,
}

/// Prefer an already-installed VLS over downloading one.
fn find_local_vls(worktree: &zed::Worktree) -> Option<String> {
	if let Ok(explicit) = std::env::var("VLS_PATH") {
		let path = explicit.trim();
		if !path.is_empty() && fs::metadata(path).map(|s| s.is_file()).unwrap_or(false) {
			return Some(path.to_string());
		}
	}

	if let Some(path) = worktree.which("vls") {
		return Some(path);
	}
	// Windows installs often use vls.exe even when which("vls") is picky.
	if let Some(path) = worktree.which("vls.exe") {
		return Some(path);
	}

	// Keep a copy next to the V compiler discoverable without PATH changes.
	if let Some(v) = find_v_compiler(worktree) {
		if let Some(dir) = parent_dir(&v) {
			for name in ["vls", "vls.exe"] {
				let candidate = join_path(&dir, name);
				if fs::metadata(&candidate).map(|s| s.is_file()).unwrap_or(false) {
					return Some(candidate);
				}
			}
		}
	}

	None
}

fn find_v_compiler(worktree: &zed::Worktree) -> Option<String> {
	if let Ok(explicit) = std::env::var("VLS_V_COMMAND") {
		let path = explicit.trim();
		if !path.is_empty() && fs::metadata(path).map(|s| s.is_file()).unwrap_or(false) {
			return Some(path.to_string());
		}
	}
	if let Some(path) = worktree.which("v") {
		return Some(path);
	}
	if let Some(path) = worktree.which("v.exe") {
		return Some(path);
	}
	None
}

fn parent_dir(path: &str) -> Option<String> {
	let idx = path.rfind(['/', '\\'])?;
	if idx == 0 {
		return Some(path[..1].to_string());
	}
	Some(path[..idx].to_string())
}

fn join_path(dir: &str, name: &str) -> String {
	if dir.ends_with('/') || dir.ends_with('\\') {
		format!("{dir}{name}")
	} else if dir.contains('\\') {
		format!("{dir}\\{name}")
	} else {
		format!("{dir}/{name}")
	}
}

fn download_vls(
	selff: &mut VExtension,
	language_server_id: &LanguageServerId,
) -> Result<String> {
	let (platform, arch) = zed::current_platform();
	zed::set_language_server_installation_status(
		language_server_id,
		&zed::LanguageServerInstallationStatus::CheckingForUpdate,
	);

	let asset_name = format!(
		"vls-{os}-{arch}{extension}",
		arch = match arch {
			zed::Architecture::Aarch64 => "arm64",
			zed::Architecture::X86 => "x86",
			zed::Architecture::X8664 => "x86_64",
		},
		os = match platform {
			zed::Os::Mac => "darwin",
			zed::Os::Linux => "linux",
			zed::Os::Windows => "windows",
		},
		extension = match platform {
			zed::Os::Windows => ".exe",
			_ => "",
		},
	);

	let release = zed::latest_github_release(
		"lv37/vls",
		zed::GithubReleaseOptions {
			require_assets: true,
			pre_release: false,
		},
	)?;

	let asset = release
		.assets
		.iter()
		.find(|asset| asset.name == asset_name)
		.ok_or_else(|| format!("no asset found matching {:?}", asset_name))?;

	if selff.current_version != asset.download_url
		|| !fs::metadata(&asset_name).map_or(false, |stat| stat.is_file())
	{
		zed::set_language_server_installation_status(
			language_server_id,
			&zed::LanguageServerInstallationStatus::Downloading,
		);

		zed::download_file(
			&asset.download_url,
			&asset_name,
			zed::DownloadedFileType::Uncompressed,
		)
		.map_err(|e| format!("failed to download file: {e}"))?;

		zed::make_file_executable(&asset_name)?;

		let entries =
			fs::read_dir(".").map_err(|e| format!("failed to list working directory {e}"))?;
		for entry in entries {
			let entry = entry.map_err(|e| format!("failed to load directory entry {e}"))?;
			if entry.file_name().to_str() != Some(&asset_name) {
				fs::remove_dir_all(&entry.path()).ok();
			}
		}
	}

	selff.cached_binary_path = Some(asset_name.clone());
	selff.current_version = release.version;
	Ok(asset_name)
}

impl VExtension {
	fn language_server_binary_path(
		&mut self,
		language_server_id: &LanguageServerId,
		worktree: &zed::Worktree,
	) -> Result<String> {
		if let Some(local) = find_local_vls(worktree) {
			return Ok(local);
		}

		if let Some(cache) = selff_cached_path(self) {
			if fs::metadata(&cache).map_or(false, |stat| stat.is_file()) {
				return Ok(cache);
			}
		}

		download_vls(self, language_server_id)
	}
}

fn selff_cached_path(selff: &VExtension) -> Option<String> {
	selff.cached_binary_path.clone()
}

impl zed::Extension for VExtension {
	fn new() -> Self {
		Self {
			cached_binary_path: None,
			current_version: "".to_string(),
		}
	}

	fn language_server_command(
		&mut self,
		language_server_id: &LanguageServerId,
		worktree: &zed::Worktree,
	) -> Result<zed::Command> {
		let command = self.language_server_binary_path(language_server_id, worktree)?;

		// VLS shells out to `v`. Pass an explicit compiler path when we can
		// find one, so diagnostics/completion work even if `v` is not on the
		// editor process PATH.
		let mut env = Vec::new();
		if let Some(v) = find_v_compiler(worktree) {
			env.push(("VLS_V_COMMAND".to_string(), v));
		}

		Ok(zed::Command {
			command,
			args: vec![],
			env,
		})
	}

	fn label_for_completion(
		&self,
		_language_server_id: &LanguageServerId,
		completion: zed::lsp::Completion,
	) -> Option<zed::CodeLabel> {
		let (highlight_name, label) = match completion.kind {
			Some(zed::lsp::CompletionKind::Struct) => ("type", completion.label),
			Some(zed::lsp::CompletionKind::Interface) => ("type", completion.label),
			Some(zed::lsp::CompletionKind::Function) => ("function", completion.label),
			Some(zed::lsp::CompletionKind::Method) => ("function", completion.label),
			_ => ("identifier", completion.label),
		};

		Some(CodeLabel {
			spans: vec![
				Some(CodeLabelSpan::Literal(CodeLabelSpanLiteral {
					text: label.clone(),
					highlight_name: Some(String::from(highlight_name)),
				})),
				completion.detail.map(|detail| {
					CodeLabelSpan::Literal(CodeLabelSpanLiteral {
						text: format!(" {}", detail),
						highlight_name: Some(String::from("type")),
					})
				}),
			]
			.into_iter()
			.flatten()
			.collect(),
			filter_range: (0..label.len()).into(),
			code: label,
		})
	}
}

zed::register_extension!(VExtension);
