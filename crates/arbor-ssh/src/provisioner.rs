use {
    crate::connection::SshConnection,
    arbor_core::{
        outpost::RemoteHost,
        remote::{ProvisionResult, RemoteError, RemoteProvisioner, RemoteTransport},
    },
};

pub struct SshProvisioner<'a> {
    connection: &'a SshConnection,
    host: &'a RemoteHost,
}

impl<'a> SshProvisioner<'a> {
    pub fn new(connection: &'a SshConnection, host: &'a RemoteHost) -> Self {
        Self { connection, host }
    }
}

impl SshProvisioner<'_> {
    /// Provision with a progress callback that receives status messages
    /// for UI display (e.g. "Connecting over SSH…", "Cloning repository…").
    pub fn provision_with_progress(
        &self,
        clone_url: &str,
        outpost_label: &str,
        branch: &str,
        on_progress: impl Fn(&str),
    ) -> Result<ProvisionResult, RemoteError> {
        on_progress("Creating remote directory…");
        self.provision_inner(clone_url, outpost_label, branch, &on_progress)
    }

    fn provision_inner(
        &self,
        clone_url: &str,
        outpost_label: &str,
        branch: &str,
        on_progress: &dyn Fn(&str),
    ) -> Result<ProvisionResult, RemoteError> {
        let base_path = &self.host.remote_base_path;
        let dir_name = sanitize_outpost_dir_name(outpost_label);
        let remote_path = format!("{base_path}/{dir_name}");
        let escaped_path = shell_escape(&remote_path);
        let escaped_url = shell_escape(clone_url);
        let escaped_branch = shell_escape(branch);

        let mkdir_cmd = format!("mkdir -p {escaped_path}");
        tracing::info!(cmd = mkdir_cmd.as_str(), "running remote command");
        let mkdir_output = self.connection.run_command(&mkdir_cmd)?;
        if mkdir_output.exit_code != Some(0) {
            tracing::error!(
                cmd = mkdir_cmd.as_str(),
                stderr = mkdir_output.stderr.as_str(),
                "mkdir failed on remote host"
            );
            return Err(RemoteError::Command(format!(
                "failed to create remote directory: {}",
                mkdir_output.stderr,
            )));
        }

        let check_cmd = format!("test -d {escaped_path}/.git && echo exists");
        tracing::info!(cmd = check_cmd.as_str(), "running remote command");
        let check_output = self.connection.run_command(&check_cmd)?;
        let already_cloned = check_output.stdout.trim() == "exists";

        if !already_cloned {
            on_progress("Cloning repository…");
            // Clone the default branch first, then create the new branch.
            // Using --branch would fail if the branch doesn't exist on the
            // remote yet (which is the common case for new outposts).
            let clone_cmd = format!(
                "GIT_SSH_COMMAND='ssh -F /dev/null' git clone {escaped_url} {escaped_path}"
            );
            tracing::info!(
                cmd = clone_cmd.as_str(),
                clone_url,
                branch,
                remote_path,
                "cloning repository on remote host"
            );
            #[cfg(unix)]
            let clone_output = self
                .connection
                .run_command_with_agent_forwarding(&clone_cmd)?;
            #[cfg(not(unix))]
            let clone_output = self.connection.run_command(&clone_cmd)?;
            if clone_output.exit_code != Some(0) {
                tracing::error!(
                    cmd = clone_cmd.as_str(),
                    clone_url,
                    branch,
                    remote_path,
                    stderr = clone_output.stderr.as_str(),
                    "git clone failed on remote host"
                );
                return Err(RemoteError::Command(format!(
                    "git clone failed: {}",
                    clone_output.stderr,
                )));
            }
        }

        on_progress("Checking out branch…");
        // Create and switch to the target branch.  If it already exists
        // (e.g. the repo was already cloned), just check it out.
        let checkout_cmd = format!(
            "cd {escaped_path} && \
             git checkout {escaped_branch} 2>/dev/null || git checkout -b {escaped_branch}"
        );
        tracing::info!(cmd = checkout_cmd.as_str(), "running remote command");
        let checkout_output = self.connection.run_command(&checkout_cmd)?;
        if checkout_output.exit_code != Some(0) {
            tracing::error!(
                cmd = checkout_cmd.as_str(),
                branch,
                remote_path,
                stderr = checkout_output.stderr.as_str(),
                "branch checkout failed on remote host"
            );
            return Err(RemoteError::Command(format!(
                "branch checkout failed: {}",
                checkout_output.stderr,
            )));
        }

        on_progress("Detecting remote daemon…");
        let has_remote_daemon = detect_remote_daemon(self.connection, self.host);

        Ok(ProvisionResult {
            remote_path,
            has_remote_daemon,
        })
    }
}

impl RemoteProvisioner for SshProvisioner<'_> {
    fn provision(
        &self,
        clone_url: &str,
        outpost_label: &str,
        branch: &str,
    ) -> Result<ProvisionResult, RemoteError> {
        self.provision_inner(clone_url, outpost_label, branch, &|_| {})
    }
}

/// Sanitise an outpost label into a safe directory name by replacing
/// whitespace and shell-unfriendly characters with hyphens.
pub fn sanitize_outpost_dir_name(label: &str) -> String {
    let mut out = String::with_capacity(label.len());
    let mut prev_dash = false;
    for ch in label.trim().chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
            out.push(ch);
            prev_dash = false;
        } else if !prev_dash && !out.is_empty() {
            out.push('-');
            prev_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        "outpost".to_owned()
    } else {
        out
    }
}

/// Shell-escape a value for safe interpolation into a remote command.
///
/// Uses double quotes so that `~` at the start of a path is replaced with
/// `$HOME` (which the remote shell expands), while everything else is
/// protected from word-splitting and globbing.
fn shell_escape(value: &str) -> String {
    // Inside double quotes we must escape: $ ` " \ !
    if let Some(rest) = value.strip_prefix("~/") {
        // Expand leading tilde so the remote shell resolves $HOME.
        let escaped_rest = escape_double_quote_inner(rest);
        format!("\"$HOME/{escaped_rest}\"")
    } else {
        let escaped = escape_double_quote_inner(value);
        format!("\"{escaped}\"")
    }
}

fn escape_double_quote_inner(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '$' | '`' | '"' | '\\' | '!' => {
                out.push('\\');
                out.push(ch);
            },
            _ => out.push(ch),
        }
    }
    out
}

fn detect_remote_daemon(connection: &SshConnection, host: &RemoteHost) -> bool {
    let Some(daemon_port) = host.daemon_port else {
        return false;
    };

    let check_cmd =
        format!("curl -sf http://127.0.0.1:{daemon_port}/api/sessions > /dev/null 2>&1 && echo ok");
    match connection.run_command(&check_cmd) {
        Ok(output) => output.stdout.trim() == "ok",
        Err(_) => false,
    }
}
