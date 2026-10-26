//! Redact sensitive application credentials from all external subprocesses.
//!
//! Rust-Boy invokes trusted OS binaries for SSH, diagnostics and HTTPS. Those
//! binaries do not need API credentials from the launcher environment. Avoid
//! sending them unrelated secrets even when the binary exits unsuccessfully.
//! This is not a sandbox: PATH substitution and arbitrary environment variables
//! outside this explicit list require independent operational controls.

use std::ffi::OsStr;
use std::process::Command;

/// Known credentials unrelated to SSH, ICMP, ARP, browser launch or Census.
pub const SENSITIVE_CHILD_ENV: &[&str] = &[
    "RUSTBOY_UNIFI_API_KEY",
    "GH_TOKEN",
    "GITHUB_TOKEN",
    "OPENAI_API_KEY",
    "ANTHROPIC_API_KEY",
    "AWS_ACCESS_KEY_ID",
    "AWS_SECRET_ACCESS_KEY",
    "AWS_SESSION_TOKEN",
    "AZURE_CLIENT_SECRET",
    "GOOGLE_API_KEY",
    "HF_TOKEN",
];

/// Create an external process with recognized credentials explicitly removed.
///
/// Other environment values, including PATH and platform certificate settings,
/// remain available so existing native tools retain their expected behavior.
pub fn isolated_command(program: impl AsRef<OsStr>) -> Command {
    let mut command = Command::new(program);
    for &key in SENSITIVE_CHILD_ENV {
        command.env_remove(key);
    }
    command
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognized_credentials_are_removed_from_all_child_processes() {
        let command = isolated_command("rust-boy-nonexistent-test");
        for &key in SENSITIVE_CHILD_ENV {
            assert!(
                command
                    .get_envs()
                    .any(|(name, value)| name == OsStr::new(key) && value.is_none()),
                "missing sensitive environment removal for {key}"
            );
        }
    }

    #[test]
    fn unrelated_parent_environment_is_not_cleared() {
        let command = isolated_command("rust-boy-nonexistent-test");
        // The helper does not clear PATH or OS-specific transport/certificate
        // configuration. No child is executed by this test.
        assert!(!command
            .get_envs()
            .any(|(name, _)| name == OsStr::new("PATH")));
    }
}
