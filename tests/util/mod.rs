pub fn test_command() -> TestCommandBuilder {
    TestCommandBuilder {
        options: Vec::new(),
        files: Vec::new(),
    }
}

pub struct TestCommandBuilder {
    options: Vec<String>,
    files: Vec<String>,
}

impl TestCommandBuilder {
    pub fn option(mut self, option: &str) -> TestCommandBuilder {
        self.options.push(option.to_string());
        self
    }

    pub fn options(mut self, options: &[&str]) -> TestCommandBuilder {
        self.options
            .extend(options.into_iter().map(|s| s.to_string()));
        self
    }

    pub fn file(mut self, file: &str) -> TestCommandBuilder {
        self.files.push(file.to_string());
        self
    }

    pub fn build(self) -> assert_cmd::Command {
        let mut args = Vec::new();
        args.extend(self.options);
        args.extend(self.files);

        let mut command = assert_cmd::Command::cargo_bin("rut").unwrap();

        command.args(args);
        command
    }
}
