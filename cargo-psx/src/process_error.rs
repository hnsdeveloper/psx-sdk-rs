#[derive(Debug)]
pub struct ProcessError {
    cmd: String,
    args: Option<String>,
    exit_code: i32,
}

impl ProcessError {
    pub fn new(cmd: String, args: Option<String>, exit_code: i32) -> Self {
        Self {
            cmd,
            args,
            exit_code,
        }
    }
}

impl std::fmt::Display for ProcessError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(args) = &self.args {
            write!(
                formatter,
                "Command {} {} exited with code {}",
                self.cmd, args, self.exit_code
            )
        } else {
            write!(
                formatter,
                "Command {} exited with code {}",
                self.cmd, self.exit_code
            )
        }
    }
}

impl std::error::Error for ProcessError {}
