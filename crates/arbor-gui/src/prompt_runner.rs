#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PromptExecutionMode {
    CaptureOutput,
    TerminalSession,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PromptExecutionPlan {
    shell_command: String,
}

fn build_prompt_execution_plan(
    preset: AgentPresetKind,
    configured_command: &str,
    prompt: &str,
    mode: PromptExecutionMode,
) -> Result<PromptExecutionPlan, String> {
    let configured_command = configured_command.trim();
    if configured_command.is_empty() {
        return Err(format!("{} preset command is empty", preset.label()));
    }

    let prompt = prompt.trim();
    if prompt.is_empty() {
        return Err("prompt cannot be empty".to_owned());
    }

    let shell_command = match mode {
        PromptExecutionMode::CaptureOutput => match preset {
            AgentPresetKind::Claude => {
                format!("{configured_command} --print {}", shell_quote(prompt))
            },
            AgentPresetKind::Codex => {
                format!("{configured_command} exec {}", shell_quote(prompt))
            },
            AgentPresetKind::OpenCode => {
                format!("{configured_command} run {}", shell_quote(prompt))
            },
            AgentPresetKind::Copilot => {
                format!(
                    "{configured_command} --allow-all-tools -p {} -s",
                    shell_quote(prompt)
                )
            },
            AgentPresetKind::Pi => {
                return Err(format!(
                    "{} does not support non-interactive prompt execution yet",
                    preset.label()
                ));
            },
        },
        PromptExecutionMode::TerminalSession => {
            format!("{configured_command} {}", shell_quote(prompt))
        },
    };

    Ok(PromptExecutionPlan { shell_command })
}

fn run_prompt_capture(
    worktree_path: &Path,
    preset: AgentPresetKind,
    configured_command: &str,
    prompt: &str,
    operation: &str,
) -> Result<String, String> {
    let plan = build_prompt_execution_plan(
        preset,
        configured_command,
        prompt,
        PromptExecutionMode::CaptureOutput,
    )?;
    let mut command = shell_expression_command(&plan.shell_command);
    command.current_dir(worktree_path);

    let output = run_command_output(&mut command, operation)?;
    if !output.status.success() {
        return Err(command_failure_message(operation, &output));
    }

    let text = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if text.is_empty() {
        return Err(format!("{operation} returned empty output"));
    }

    Ok(text)
}

fn prompt_terminal_invocation(
    preset: AgentPresetKind,
    configured_command: &str,
    prompt: &str,
) -> Result<String, String> {
    build_prompt_execution_plan(
        preset,
        configured_command,
        prompt,
        PromptExecutionMode::TerminalSession,
    )
    .map(|plan| plan.shell_command)
}

#[cfg(target_os = "windows")]
fn shell_expression_command(expression: &str) -> Command {
    let mut command = create_command("cmd");
    command.arg("/C").arg(expression);
    command
}

#[cfg(not(target_os = "windows"))]
fn shell_expression_command(expression: &str) -> Command {
    let mut command = create_command("sh");
    command.arg("-lc").arg(expression);
    command
}

#[cfg(test)]
mod prompt_runner_tests {
    use super::*;

    #[test]
    fn codex_capture_plan_uses_exec_mode() {
        let plan = build_prompt_execution_plan(
            AgentPresetKind::Codex,
            "codex --model gpt-5",
            "summarize the diff",
            PromptExecutionMode::CaptureOutput,
        )
        .unwrap_or_else(|error| panic!("plan should build: {error}"));

        assert!(plan.shell_command.contains("codex --model gpt-5 exec "));
    }

    #[test]
    fn pi_capture_plan_is_not_supported() {
        let error = build_prompt_execution_plan(
            AgentPresetKind::Pi,
            "pi",
            "summarize the diff",
            PromptExecutionMode::CaptureOutput,
        )
        .err()
        .unwrap_or_else(|| panic!("pi capture should be unsupported"));

        assert!(error.contains("Pi does not support non-interactive prompt execution yet"));
    }

    #[test]
    fn copilot_capture_plan_uses_prompt_and_silent_flags() {
        let plan = build_prompt_execution_plan(
            AgentPresetKind::Copilot,
            "copilot --model gpt-5.2",
            "summarize the diff",
            PromptExecutionMode::CaptureOutput,
        )
        .unwrap_or_else(|error| panic!("plan should build: {error}"));

        assert!(plan.shell_command.contains("copilot --model gpt-5.2 --allow-all-tools -p "));
        assert!(plan.shell_command.ends_with(" -s"));
    }

    #[test]
    fn terminal_plan_quotes_prompt() {
        let plan = build_prompt_execution_plan(
            AgentPresetKind::Claude,
            "claude --dangerously-skip-permissions",
            "review branch named it's-ready",
            PromptExecutionMode::TerminalSession,
        )
        .unwrap_or_else(|error| panic!("plan should build: {error}"));

        assert!(plan.shell_command.contains("claude --dangerously-skip-permissions "));
        assert!(plan.shell_command.contains("it'\"'\"'s-ready"));
    }
}
