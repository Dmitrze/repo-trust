//! Repository URL parsing and normalization.

use anyhow::{anyhow, Context};

/// Parse `owner/repo` or a full GitHub URL into the canonical `owner/repo` form.
pub fn parse(input: &str) -> anyhow::Result<String> {
    let trimmed = input.trim().trim_end_matches('/').trim_end_matches(".git");

    // Already in `owner/repo` form?
    if !trimmed.contains("://") && !trimmed.contains('@') {
        let parts: Vec<&str> = trimmed.split('/').collect();
        if parts.len() == 2 && !parts[0].is_empty() && !parts[1].is_empty() {
            return Ok(format!("{}/{}", parts[0], parts[1]));
        }
    }

    let url = url::Url::parse(trimmed).with_context(|| format!("could not parse '{trimmed}' as URL"))?;

    let host = url.host_str().ok_or_else(|| anyhow!("URL has no host"))?;
    if !host.eq_ignore_ascii_case("github.com") && !host.eq_ignore_ascii_case("www.github.com") {
        anyhow::bail!("only github.com URLs are supported (got {host})");
    }

    let path = url.path().trim_start_matches('/').trim_end_matches('/');
    let segments: Vec<&str> = path.split('/').collect();
    if segments.len() < 2 {
        anyhow::bail!("URL path must contain owner/repo: {trimmed}");
    }

    Ok(format!("{}/{}", segments[0], segments[1]))
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn parses_owner_repo() {
        assert_eq!(parse("octocat/Hello-World").unwrap(), "octocat/Hello-World");
    }

    #[test]
    fn parses_full_url() {
        assert_eq!(
            parse("https://github.com/octocat/Hello-World").unwrap(),
            "octocat/Hello-World"
        );
    }

    #[test]
    fn parses_url_with_git_suffix() {
        assert_eq!(
            parse("https://github.com/octocat/Hello-World.git").unwrap(),
            "octocat/Hello-World"
        );
    }

    #[test]
    fn parses_url_with_trailing_slash() {
        assert_eq!(
            parse("https://github.com/octocat/Hello-World/").unwrap(),
            "octocat/Hello-World"
        );
    }

    #[test]
    fn rejects_non_github_host() {
        assert!(parse("https://gitlab.com/owner/repo").is_err());
    }

    #[test]
    fn rejects_empty_owner_or_repo() {
        assert!(parse("/").is_err());
        assert!(parse("owner/").is_err());
    }
}
