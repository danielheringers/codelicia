use codex_git::merge_base_with_head;
use codex_protocol::protocol::ReviewRequest;
use codex_protocol::protocol::ReviewTarget;
use std::path::Path;

#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedReviewRequest {
    pub target: ReviewTarget,
    pub prompt: String,
    pub user_facing_hint: String,
}

const UNCOMMITTED_PROMPT: &str = "Review the current code changes (staged, unstaged, and untracked files) and provide prioritized findings.";

const BASE_BRANCH_PROMPT_BACKUP: &str = "Review the code changes against the base branch '{branch}'. Start by finding the merge diff between the current branch and {branch}'s upstream e.g. (`git merge-base HEAD \"$(git rev-parse --abbrev-ref \"{branch}@{upstream}\")\"`), then run `git diff` against that SHA to see what changes we would merge into the {branch} branch. Provide prioritized, actionable findings.";
const BASE_BRANCH_PROMPT: &str = "Review the code changes against the base branch '{baseBranch}'. The merge base commit for this comparison is {mergeBaseSha}. Run `git diff {mergeBaseSha}` to inspect the changes relative to {baseBranch}. Provide prioritized, actionable findings.";

const COMMIT_PROMPT_WITH_TITLE: &str = "Review the code changes introduced by commit {sha} (\"{title}\"). Provide prioritized, actionable findings.";
const COMMIT_PROMPT: &str =
    "Review the code changes introduced by commit {sha}. Provide prioritized, actionable findings.";

const FILES_PROMPT_PREFIX: &str = "Review only the listed files and keep the scope strictly limited to them. Do not review unrelated files unless you explicitly call out why broader context is required.";

pub fn resolve_review_request(
    request: ReviewRequest,
    cwd: &Path,
) -> anyhow::Result<ResolvedReviewRequest> {
    let target = request.target;
    let prompt = review_prompt(&target, cwd)?;
    let user_facing_hint = request
        .user_facing_hint
        .unwrap_or_else(|| user_facing_hint(&target));

    Ok(ResolvedReviewRequest {
        target,
        prompt,
        user_facing_hint,
    })
}

pub fn review_prompt(target: &ReviewTarget, cwd: &Path) -> anyhow::Result<String> {
    match target {
        ReviewTarget::UncommittedChanges => Ok(UNCOMMITTED_PROMPT.to_string()),
        ReviewTarget::BaseBranch { branch } => {
            if let Some(commit) = merge_base_with_head(cwd, branch)? {
                Ok(BASE_BRANCH_PROMPT
                    .replace("{baseBranch}", branch)
                    .replace("{mergeBaseSha}", &commit))
            } else {
                Ok(BASE_BRANCH_PROMPT_BACKUP.replace("{branch}", branch))
            }
        }
        ReviewTarget::Commit { sha, title } => {
            if let Some(title) = title {
                Ok(COMMIT_PROMPT_WITH_TITLE
                    .replace("{sha}", sha)
                    .replace("{title}", title))
            } else {
                Ok(COMMIT_PROMPT.replace("{sha}", sha))
            }
        }
        ReviewTarget::Custom { instructions } => {
            let prompt = instructions.trim();
            if prompt.is_empty() {
                anyhow::bail!("Review prompt cannot be empty");
            }
            Ok(prompt.to_string())
        }
        ReviewTarget::Files { paths } => {
            let cleaned_paths: Vec<String> = paths
                .iter()
                .map(|path| path.trim())
                .filter(|path| !path.is_empty())
                .map(ToString::to_string)
                .collect();

            if cleaned_paths.is_empty() {
                anyhow::bail!("Review file paths must contain at least one non-empty path");
            }

            let listed_paths = cleaned_paths
                .iter()
                .map(|path| format!("- {path}"))
                .collect::<Vec<_>>()
                .join("\n");
            Ok(format!(
                "{FILES_PROMPT_PREFIX}\n\nTarget files:\n{listed_paths}"
            ))
        }
    }
}

pub fn user_facing_hint(target: &ReviewTarget) -> String {
    match target {
        ReviewTarget::UncommittedChanges => "current changes".to_string(),
        ReviewTarget::BaseBranch { branch } => format!("changes against '{branch}'"),
        ReviewTarget::Commit { sha, title } => {
            let short_sha: String = sha.chars().take(7).collect();
            if let Some(title) = title {
                format!("commit {short_sha}: {title}")
            } else {
                format!("commit {short_sha}")
            }
        }
        ReviewTarget::Custom { instructions } => instructions.trim().to_string(),
        ReviewTarget::Files { paths } => {
            let cleaned_paths: Vec<&str> = paths
                .iter()
                .map(|path| path.trim())
                .filter(|path| !path.is_empty())
                .collect();

            match cleaned_paths.len() {
                0 => "files".to_string(),
                1 => format!("file {}", cleaned_paths[0]),
                _ => format!("{} files", cleaned_paths.len()),
            }
        }
    }
}

impl From<ResolvedReviewRequest> for ReviewRequest {
    fn from(resolved: ResolvedReviewRequest) -> Self {
        ReviewRequest {
            target: resolved.target,
            user_facing_hint: Some(resolved.user_facing_hint),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn review_prompt_files_rejects_when_no_non_empty_paths() {
        let err = review_prompt(
            &ReviewTarget::Files {
                paths: vec!["".to_string(), "   ".to_string()],
            },
            Path::new("."),
        )
        .expect_err("expected validation error");

        assert!(
            err.to_string()
                .contains("must contain at least one non-empty path")
        );
    }

    #[test]
    fn review_prompt_files_uses_trimmed_non_empty_paths_only() {
        let prompt = review_prompt(
            &ReviewTarget::Files {
                paths: vec![
                    " src/lib.rs ".to_string(),
                    "".to_string(),
                    "\t".to_string(),
                    "src/main.rs".to_string(),
                ],
            },
            Path::new("."),
        )
        .expect("prompt should be generated");

        assert!(prompt.contains("Target files:"));
        assert!(prompt.contains("- src/lib.rs"));
        assert!(prompt.contains("- src/main.rs"));
        assert!(!prompt.contains("- \t"));
    }

    #[test]
    fn user_facing_hint_files_reflects_file_scope() {
        assert_eq!(
            user_facing_hint(&ReviewTarget::Files {
                paths: vec![" src/lib.rs ".to_string(), "".to_string()],
            }),
            "file src/lib.rs"
        );

        assert_eq!(
            user_facing_hint(&ReviewTarget::Files {
                paths: vec!["a.rs".to_string(), "b.rs".to_string()],
            }),
            "2 files"
        );
    }
}
