use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use url::Url;

use crate::template::parser::parse_template_hcl;
use crate::template::schema::TemplateFile;

#[derive(Debug, serde::Serialize)]
pub struct TemplateImportResult {
    pub source_ref: String,
    pub source: String,
    pub destination: String,
    /// Names (keys) of secrets that the imported template declares as required.
    /// These are identifiers like `"GITHUB_TOKEN"`, not actual secret values.
    #[serde(rename = "required_secrets")]
    pub required_credential_names: Vec<String>,
}

pub async fn import_template_from_source_ref(
    cwd: &Path,
    source_ref: &str,
    destination_dir: &Path,
) -> Result<TemplateImportResult> {
    let source = parse_template_source_ref(source_ref)?;
    let source_path = resolve_source_for_display(cwd, &source);
    let file_name = template_file_name(cwd, &source)?;
    let source_bytes = read_source_bytes(cwd, &source).await?;

    fs::create_dir_all(destination_dir).with_context(|| {
        format!(
            "failed creating template directory {}",
            destination_dir.display()
        )
    })?;

    let destination_path = destination_dir.join(&file_name);
    if destination_path.exists() {
        bail!(
            "template destination already exists: {}",
            destination_path.display()
        );
    }

    fs::write(&destination_path, &source_bytes).with_context(|| {
        format!(
            "failed writing imported template to {}",
            destination_path.display()
        )
    })?;

    let required_credential_names = scan_credential_keys(&destination_path)?;

    Ok(TemplateImportResult {
        source_ref: source_ref.to_string(),
        source: source_path,
        destination: destination_path.display().to_string(),
        required_credential_names,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ParsedTemplateSourceRef {
    LocalPath(PathBuf),
    RemoteUrl(Url),
}

fn parse_template_source_ref(source_ref: &str) -> Result<ParsedTemplateSourceRef> {
    let trimmed = source_ref.trim();
    if trimmed.is_empty() {
        bail!(
            "invalid template source reference `{source_ref}`; expected a local path or an http(s) URL"
        );
    }

    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        let url =
            Url::parse(trimmed).with_context(|| format!("invalid template URL `{source_ref}`"))?;
        return Ok(ParsedTemplateSourceRef::RemoteUrl(url));
    }

    if trimmed.contains("://") {
        bail!(
            "unsupported template URL scheme in `{source_ref}`; only http:// and https:// are supported"
        );
    }

    Ok(ParsedTemplateSourceRef::LocalPath(PathBuf::from(trimmed)))
}

fn resolve_source_for_display(cwd: &Path, source: &ParsedTemplateSourceRef) -> String {
    match source {
        ParsedTemplateSourceRef::LocalPath(path) => {
            let resolved = resolve_local_source_path(cwd, path);
            resolved.display().to_string()
        }
        ParsedTemplateSourceRef::RemoteUrl(url) => url.to_string(),
    }
}

fn template_file_name(cwd: &Path, source: &ParsedTemplateSourceRef) -> Result<String> {
    let raw_name = match source {
        ParsedTemplateSourceRef::LocalPath(path) => {
            let resolved = resolve_local_source_path(cwd, path);
            if !resolved.is_file() {
                bail!(
                    "template source path `{}` was not found or is not a file",
                    resolved.display()
                );
            }
            resolved
                .file_name()
                .and_then(|value| value.to_str())
                .map(ToOwned::to_owned)
                .context("template source path must include a file name")?
        }
        ParsedTemplateSourceRef::RemoteUrl(url) => {
            let segment = url
                .path_segments()
                .and_then(|mut segments| segments.rfind(|segment| !segment.is_empty()))
                .context("template URL path must include a file name")?;
            segment.to_string()
        }
    };

    validate_template_file_name(&raw_name)?;
    Ok(raw_name)
}

fn validate_template_file_name(file_name: &str) -> Result<()> {
    if matches!(file_name, "." | "..") {
        bail!("template file name contains invalid segment `{file_name}`");
    }

    if file_name.contains('/') || file_name.contains('\\') {
        bail!("template file name contains invalid segment `{file_name}`");
    }

    if !file_name.ends_with(".hcl") {
        bail!("template file must end with .hcl");
    }

    Ok(())
}

async fn read_source_bytes(cwd: &Path, source: &ParsedTemplateSourceRef) -> Result<Vec<u8>> {
    match source {
        ParsedTemplateSourceRef::LocalPath(path) => {
            let resolved = resolve_local_source_path(cwd, path);
            if !resolved.is_file() {
                bail!(
                    "template source path `{}` was not found or is not a file",
                    resolved.display()
                );
            }
            fs::read(&resolved)
                .with_context(|| format!("failed reading template source {}", resolved.display()))
        }
        ParsedTemplateSourceRef::RemoteUrl(url) => {
            const MAX_TEMPLATE_SIZE: u64 = 1_048_576; // 1 MiB

            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .redirect(reqwest::redirect::Policy::limited(5))
                .build()
                .context("failed building HTTP client for template download")?;

            let response = client
                .get(url.clone())
                .send()
                .await
                .with_context(|| format!("failed downloading template from `{url}`"))?;

            if !response.status().is_success() {
                bail!(
                    "failed downloading template from `{url}`: HTTP {}",
                    response.status()
                );
            }

            if let Some(len) = response.content_length()
                && len > MAX_TEMPLATE_SIZE
            {
                bail!(
                    "template from `{url}` exceeds maximum size ({len} bytes > {MAX_TEMPLATE_SIZE} bytes)"
                );
            }

            let body = response
                .bytes()
                .await
                .with_context(|| format!("failed reading template response body from `{url}`"))?;

            if body.len() as u64 > MAX_TEMPLATE_SIZE {
                bail!(
                    "template from `{url}` exceeds maximum size ({} bytes > {MAX_TEMPLATE_SIZE} bytes)",
                    body.len()
                );
            }

            Ok(body.to_vec())
        }
    }
}

fn resolve_local_source_path(cwd: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        return path.to_path_buf();
    }
    cwd.join(path)
}

fn scan_credential_keys(template_path: &Path) -> Result<Vec<String>> {
    let content = fs::read_to_string(template_path).with_context(|| {
        format!(
            "failed reading imported template for secret scan {}",
            template_path.display()
        )
    })?;
    let base_dir = template_path.parent().unwrap_or(Path::new("."));
    let template_file: TemplateFile =
        parse_template_hcl(&content, base_dir).with_context(|| {
            format!(
                "imported template is not valid HCL/schema {}",
                template_path.display()
            )
        })?;

    Ok(collect_credential_keys(&template_file))
}

fn collect_credential_keys(template_file: &TemplateFile) -> Vec<String> {
    let mut secrets = BTreeSet::new();
    for command in template_file.commands.values() {
        for secret in &command.annotations.secrets {
            let trimmed = secret.trim();
            if !trimmed.is_empty() {
                secrets.insert(trimmed.to_string());
            }
        }
    }
    secrets.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    #[cfg(feature = "http")]
    use crate::template::schema::HttpOperationTemplate;
    use crate::template::schema::{
        Annotations, CommandMode, CommandTemplate, OperationTemplate, ResultTemplate, TemplateFile,
    };

    use super::{
        ParsedTemplateSourceRef, collect_credential_keys, parse_template_source_ref,
        validate_template_file_name,
    };

    #[test]
    fn parses_local_relative_path() {
        let parsed = parse_template_source_ref("templates/github.hcl").unwrap();
        assert_eq!(
            parsed,
            ParsedTemplateSourceRef::LocalPath("templates/github.hcl".into())
        );
    }

    #[test]
    fn parses_http_url() {
        let parsed = parse_template_source_ref("https://example.com/templates/github.hcl").unwrap();
        assert!(matches!(parsed, ParsedTemplateSourceRef::RemoteUrl(_)));
    }

    #[test]
    fn rejects_empty_reference() {
        let err = parse_template_source_ref("   ").unwrap_err();
        assert!(
            err.to_string()
                .contains("expected a local path or an http(s) URL")
        );
    }

    #[test]
    fn rejects_unsupported_url_scheme() {
        let err =
            parse_template_source_ref("git://example.com/repo/templates/github.hcl").unwrap_err();
        assert!(err.to_string().contains("unsupported template URL scheme"));
    }

    #[test]
    fn rejects_non_hcl_extension() {
        let err = validate_template_file_name("github.json").unwrap_err();
        assert!(err.to_string().contains(".hcl"));
    }

    #[test]
    fn rejects_path_traversal_segments() {
        let err = validate_template_file_name("..\\github.hcl").unwrap_err();
        assert!(err.to_string().contains("invalid segment"));
    }

    #[test]
    #[cfg(feature = "http")]
    fn collects_unique_sorted_required_secrets() {
        let mut commands = BTreeMap::new();
        commands.insert(
            "a".to_string(),
            CommandTemplate {
                title: "A".to_string(),
                summary: "A".to_string(),
                description: "A".to_string(),
                categories: vec![],
                annotations: Annotations {
                    mode: CommandMode::Read,
                    secrets: vec!["github.token".to_string(), "api.key".to_string()],
                    allow_environment_protocol_switching: false,
                },
                params: vec![],
                operation: OperationTemplate::Http(HttpOperationTemplate {
                    method: "GET".to_string(),
                    url: "https://example.com".to_string(),
                    path: None,
                    query: None,
                    headers: None,
                    cookies: None,
                    auth: None,
                    body: None,
                    stream: false,
                    transport: None,
                }),
                result: ResultTemplate {
                    decode: Default::default(),
                    extract: None,
                    output: "{{ result }}".to_string(),
                    result_alias: None,
                },
                environment_overrides: BTreeMap::new(),
            },
        );
        commands.insert(
            "b".to_string(),
            CommandTemplate {
                title: "B".to_string(),
                summary: "B".to_string(),
                description: "B".to_string(),
                categories: vec![],
                annotations: Annotations {
                    mode: CommandMode::Write,
                    secrets: vec![
                        "github.token".to_string(),
                        " ".to_string(),
                        "service.secret".to_string(),
                    ],
                    allow_environment_protocol_switching: false,
                },
                params: vec![],
                operation: OperationTemplate::Http(HttpOperationTemplate {
                    method: "GET".to_string(),
                    url: "https://example.com".to_string(),
                    path: None,
                    query: None,
                    headers: None,
                    cookies: None,
                    auth: None,
                    body: None,
                    stream: false,
                    transport: None,
                }),
                result: ResultTemplate {
                    decode: Default::default(),
                    extract: None,
                    output: "{{ result }}".to_string(),
                    result_alias: None,
                },
                environment_overrides: BTreeMap::new(),
            },
        );

        let template_file = TemplateFile {
            version: 1,
            provider: "demo".to_string(),
            categories: vec![],
            environments: None,
            commands,
        };

        let secrets = collect_credential_keys(&template_file);
        assert_eq!(
            secrets,
            vec![
                "api.key".to_string(),
                "github.token".to_string(),
                "service.secret".to_string()
            ]
        );
    }
}
