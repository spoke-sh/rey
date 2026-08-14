#![forbid(unsafe_code)]

use chrono::{DateTime, Utc};
use serde::Deserialize;
use thiserror::Error;

const GITHUB_API_PREFIX: &str = "https://api.github.com/repos/";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct GitHubNotificationThread {
    pub id: String,
    pub unread: bool,
    pub reason: String,
    pub updated_at: String,
    pub last_read_at: Option<String>,
    pub repository: GitHubRepository,
    pub subject: GitHubNotificationSubject,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct GitHubRepository {
    pub full_name: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct GitHubNotificationSubject {
    pub title: String,
    pub url: Option<String>,
    pub latest_comment_url: Option<String>,
    #[serde(rename = "type")]
    pub subject_type: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct GitHubIssueComment {
    pub id: u64,
    pub body: Option<String>,
    pub user: Option<GitHubUser>,
    pub created_at: String,
    pub updated_at: String,
    pub html_url: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct GitHubReviewComment {
    pub id: u64,
    pub body: Option<String>,
    pub user: Option<GitHubUser>,
    pub created_at: String,
    pub updated_at: String,
    pub html_url: String,
    pub path: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct GitHubUser {
    pub login: String,
}

pub fn parse_notifications(
    bytes: &[u8],
) -> Result<Vec<GitHubNotificationThread>, GitHubPollParseError> {
    let notifications: Vec<GitHubNotificationThread> = serde_json::from_slice(bytes)?;
    for notification in &notifications {
        validate_notification(notification)?;
    }
    Ok(notifications)
}

pub fn parse_issue_comments(
    bytes: &[u8],
    repository: &str,
    pull_number: u64,
) -> Result<Vec<GitHubIssueComment>, GitHubPollParseError> {
    let comments: Vec<GitHubIssueComment> = serde_json::from_slice(bytes)?;
    for comment in &comments {
        validate_comment(
            comment.id,
            comment.body.as_deref(),
            comment.user.as_ref(),
            &comment.created_at,
            &comment.updated_at,
            &comment.html_url,
        )?;
        let expected = format!(
            "https://github.com/{repository}/pull/{pull_number}#issuecomment-{}",
            comment.id
        );
        if comment.html_url != expected {
            return Err(GitHubPollParseError::InvalidCommentUrl(
                comment.html_url.clone(),
            ));
        }
    }
    Ok(comments)
}

pub fn parse_review_comments(
    bytes: &[u8],
    repository: &str,
    pull_number: u64,
) -> Result<Vec<GitHubReviewComment>, GitHubPollParseError> {
    let comments: Vec<GitHubReviewComment> = serde_json::from_slice(bytes)?;
    for comment in &comments {
        validate_comment(
            comment.id,
            comment.body.as_deref(),
            comment.user.as_ref(),
            &comment.created_at,
            &comment.updated_at,
            &comment.html_url,
        )?;
        if comment.path.is_empty() || comment.path.contains('\0') {
            return Err(GitHubPollParseError::InvalidReviewPath);
        }
        let expected = format!(
            "https://github.com/{repository}/pull/{pull_number}#discussion_r{}",
            comment.id
        );
        if comment.html_url != expected {
            return Err(GitHubPollParseError::InvalidCommentUrl(
                comment.html_url.clone(),
            ));
        }
    }
    Ok(comments)
}

pub fn parse_timestamp(value: &str) -> Result<i64, GitHubPollParseError> {
    DateTime::parse_from_rfc3339(value)
        .map(|timestamp| timestamp.with_timezone(&Utc).timestamp())
        .map_err(|_| GitHubPollParseError::InvalidTimestamp(value.to_owned()))
}

pub fn pull_request_number(
    notification: &GitHubNotificationThread,
) -> Result<Option<u64>, GitHubPollParseError> {
    if notification.subject.subject_type != "PullRequest" {
        return Ok(None);
    }
    let url = notification
        .subject
        .url
        .as_deref()
        .ok_or(GitHubPollParseError::MissingPullRequestUrl)?;
    let prefix = format!(
        "{GITHUB_API_PREFIX}{}/pulls/",
        notification.repository.full_name
    );
    let number = url
        .strip_prefix(&prefix)
        .filter(|suffix| !suffix.is_empty() && !suffix.contains('/'))
        .ok_or_else(|| GitHubPollParseError::InvalidPullRequestUrl(url.to_owned()))?
        .parse::<u64>()
        .map_err(|_| GitHubPollParseError::InvalidPullRequestUrl(url.to_owned()))?;
    if number == 0 {
        return Err(GitHubPollParseError::InvalidPullRequestUrl(url.to_owned()));
    }
    Ok(Some(number))
}

pub fn notification_html_url(
    notification: &GitHubNotificationThread,
) -> Result<String, GitHubPollParseError> {
    let repository = &notification.repository.full_name;
    match pull_request_number(notification)? {
        Some(number) => Ok(format!("https://github.com/{repository}/pull/{number}")),
        None => {
            let Some(subject_url) = notification.subject.url.as_deref() else {
                return Ok("https://github.com/notifications".to_owned());
            };
            let repository_prefix = format!("{GITHUB_API_PREFIX}{repository}/");
            let Some(relative) = subject_url.strip_prefix(&repository_prefix) else {
                return Err(GitHubPollParseError::InvalidSubjectUrl(
                    subject_url.to_owned(),
                ));
            };
            let mut parts = relative.split('/');
            match (parts.next(), parts.next(), parts.next()) {
                (Some("issues"), Some(number), None) if valid_number(number) => {
                    Ok(format!("https://github.com/{repository}/issues/{number}"))
                }
                (Some("discussions"), Some(number), None) if valid_number(number) => Ok(format!(
                    "https://github.com/{repository}/discussions/{number}"
                )),
                _ => Ok("https://github.com/notifications".to_owned()),
            }
        }
    }
}

fn validate_notification(
    notification: &GitHubNotificationThread,
) -> Result<(), GitHubPollParseError> {
    if notification.id.is_empty()
        || notification.reason.is_empty()
        || notification.subject.title.trim().is_empty()
        || notification.subject.subject_type.is_empty()
        || !valid_repository(&notification.repository.full_name)
    {
        return Err(GitHubPollParseError::InvalidNotification);
    }
    parse_timestamp(&notification.updated_at)?;
    if let Some(last_read_at) = &notification.last_read_at {
        parse_timestamp(last_read_at)?;
    }
    let repository_prefix = format!("{GITHUB_API_PREFIX}{}/", notification.repository.full_name);
    for url in [
        notification.subject.url.as_ref(),
        notification.subject.latest_comment_url.as_ref(),
    ]
    .into_iter()
    .flatten()
    {
        if !url.starts_with(&repository_prefix) {
            return Err(GitHubPollParseError::InvalidSubjectUrl(url.clone()));
        }
    }
    let _ = pull_request_number(notification)?;
    Ok(())
}

fn validate_comment(
    id: u64,
    body: Option<&str>,
    user: Option<&GitHubUser>,
    created_at: &str,
    updated_at: &str,
    html_url: &str,
) -> Result<(), GitHubPollParseError> {
    if id == 0
        || body.is_some_and(|body| body.contains('\0'))
        || user.is_some_and(|user| user.login.is_empty() || user.login.contains('\0'))
        || !html_url.starts_with("https://github.com/")
    {
        return Err(GitHubPollParseError::InvalidComment);
    }
    parse_timestamp(created_at)?;
    parse_timestamp(updated_at)?;
    Ok(())
}

fn valid_repository(repository: &str) -> bool {
    let mut parts = repository.split('/');
    matches!(
        (parts.next(), parts.next(), parts.next()),
        (Some(owner), Some(name), None)
            if !owner.is_empty()
                && !name.is_empty()
                && owner.bytes().all(valid_repository_byte)
                && name.bytes().all(valid_repository_byte)
    )
}

fn valid_repository_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-')
}

fn valid_number(value: &str) -> bool {
    value.parse::<u64>().is_ok_and(|number| number > 0)
}

#[derive(Debug, Error)]
pub enum GitHubPollParseError {
    #[error("GitHub API response is not the expected JSON relation: {0}")]
    Json(#[from] serde_json::Error),
    #[error("GitHub notification is missing a required bounded field")]
    InvalidNotification,
    #[error("GitHub notification timestamp is invalid: {0}")]
    InvalidTimestamp(String),
    #[error("GitHub notification subject URL is outside its repository boundary: {0}")]
    InvalidSubjectUrl(String),
    #[error("GitHub pull request notification has no subject URL")]
    MissingPullRequestUrl,
    #[error("GitHub pull request subject URL is invalid: {0}")]
    InvalidPullRequestUrl(String),
    #[error("GitHub comment is missing a required bounded field")]
    InvalidComment,
    #[error("GitHub comment URL does not match its exact pull request and comment identity: {0}")]
    InvalidCommentUrl(String),
    #[error("GitHub review comment path is invalid")]
    InvalidReviewPath,
}

#[cfg(test)]
mod tests {
    use super::{
        notification_html_url, parse_issue_comments, parse_notifications, pull_request_number,
    };

    #[test]
    fn parses_exact_pull_request_notification_identity() {
        let rows = parse_notifications(
            br#"[{"id":"42","unread":true,"reason":"comment","updated_at":"2026-08-13T18:00:00Z","last_read_at":"2026-08-13T17:00:00Z","repository":{"full_name":"spoke-sh/rey"},"subject":{"title":"Keep mail typed","url":"https://api.github.com/repos/spoke-sh/rey/pulls/7","latest_comment_url":"https://api.github.com/repos/spoke-sh/rey/issues/comments/9","type":"PullRequest"}}]"#,
        )
        .unwrap();

        assert_eq!(pull_request_number(&rows[0]).unwrap(), Some(7));
        assert_eq!(
            notification_html_url(&rows[0]).unwrap(),
            "https://github.com/spoke-sh/rey/pull/7"
        );
    }

    #[test]
    fn rejects_cross_repository_pull_request_subjects() {
        let error = parse_notifications(
            br#"[{"id":"42","unread":true,"reason":"comment","updated_at":"2026-08-13T18:00:00Z","last_read_at":null,"repository":{"full_name":"spoke-sh/rey"},"subject":{"title":"Wrong binding","url":"https://api.github.com/repos/other/repo/pulls/7","latest_comment_url":null,"type":"PullRequest"}}]"#,
        )
        .unwrap_err();

        assert!(error.to_string().contains("repository boundary"));
    }

    #[test]
    fn derives_issue_links_and_rejects_cross_repository_metadata() {
        let rows = parse_notifications(
            br#"[{"id":"43","unread":true,"reason":"mention","updated_at":"2026-08-13T18:00:00Z","last_read_at":null,"repository":{"full_name":"spoke-sh/rey"},"subject":{"title":"Mailbox contract","url":"https://api.github.com/repos/spoke-sh/rey/issues/8","latest_comment_url":null,"type":"Issue"}}]"#,
        )
        .unwrap();
        assert_eq!(
            notification_html_url(&rows[0]).unwrap(),
            "https://github.com/spoke-sh/rey/issues/8"
        );

        let error = parse_notifications(
            br#"[{"id":"43","unread":true,"reason":"mention","updated_at":"2026-08-13T18:00:00Z","last_read_at":null,"repository":{"full_name":"spoke-sh/rey"},"subject":{"title":"Wrong binding","url":"https://api.github.com/repos/other/repo/issues/8","latest_comment_url":null,"type":"Issue"}}]"#,
        )
        .unwrap_err();
        assert!(error.to_string().contains("repository boundary"));
    }

    #[test]
    fn rejects_comment_links_outside_the_exact_pull_request() {
        let error = parse_issue_comments(
            br#"[{"id":91,"body":"Wrong link","user":{"login":"octocat"},"created_at":"2026-08-13T17:30:00Z","updated_at":"2026-08-13T17:31:00Z","html_url":"https://github.com/other/repo/pull/7#issuecomment-91"}]"#,
            "spoke-sh/rey",
            7,
        )
        .unwrap_err();

        assert!(error.to_string().contains("exact pull request"));
    }
}
