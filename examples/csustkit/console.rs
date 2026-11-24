use std::{
    env,
    fmt::Display,
    io::{self, Write},
    ops::RangeInclusive,
    process::ExitCode,
};

#[derive(Default)]
pub struct Console {
    input_end: Option<ExitCode>,
}

impl Console {
    pub fn exit_code(&self) -> ExitCode {
        self.input_end.unwrap_or(ExitCode::SUCCESS)
    }

    pub fn text(&mut self, message: &str) -> Option<String> {
        loop {
            let input = self.prompt(message)?;
            if !input.is_empty() {
                return Some(input);
            }
            println!("输入不能为空，请重新输入。");
        }
    }

    pub fn username(&mut self, env_key: &str, message: &str) -> Option<String> {
        env::var(env_key)
            .ok()
            .filter(|value| !value.trim().is_empty())
            .or_else(|| self.text(message))
    }

    pub fn password(&mut self, env_key: &str, message: &str) -> Option<String> {
        if let Ok(password) = env::var(env_key)
            && !password.is_empty()
        {
            return Some(password);
        }
        while self.input_end.is_none() {
            match rpassword::prompt_password(format!("{message}：")) {
                Ok(password) if !password.trim().is_empty() => return Some(password),
                Ok(_) => println!("输入不能为空，请重新输入。"),
                Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => {
                    self.input_end = Some(ExitCode::SUCCESS);
                }
                Err(error) => self.input_error(error),
            }
        }
        None
    }

    pub fn number(&mut self, message: &str, range: RangeInclusive<i64>) -> Option<i64> {
        let message = format!("{message}（{}–{}，0 返回）", range.start(), range.end());
        loop {
            let input = self.prompt(&message)?;
            if input == "0" {
                return None;
            }
            if let Ok(value) = input.parse::<i64>()
                && range.contains(&value)
            {
                return Some(value);
            }
            println!("输入无效，请重新输入。");
        }
    }

    pub fn menu<T: Copy>(&mut self, title: &str, items: &[(T, &str)], back: &str) -> Option<T> {
        self.choose(title, items, |(_, label)| *label, back)
            .map(|(value, _)| *value)
    }

    pub fn select<'a, T, L: Display>(
        &mut self,
        title: &str,
        items: &'a [T],
        label: impl Fn(&'a T) -> L,
    ) -> Option<&'a T> {
        self.choose(title, items, label, "返回")
    }

    fn choose<'a, T, L: Display>(
        &mut self,
        title: &str,
        items: &'a [T],
        label: impl Fn(&'a T) -> L,
        back: &str,
    ) -> Option<&'a T> {
        if self.input_end.is_some() || items.is_empty() {
            return None;
        }
        println!("\n=== {title} ===");
        for (index, item) in items.iter().enumerate() {
            println!("{}. {}", index + 1, label(item));
        }
        println!("0. {back}");

        loop {
            let input = self.prompt("请选择")?;
            if input == "0" {
                return None;
            }
            if let Ok(number) = input.parse::<usize>()
                && let Some(index) = number.checked_sub(1)
                && let Some(item) = items.get(index)
            {
                return Some(item);
            }
            println!("输入无效，请重新选择。");
        }
    }

    fn prompt(&mut self, message: &str) -> Option<String> {
        if self.input_end.is_some() {
            return None;
        }

        let mut line = String::new();
        let mut stdout = io::stdout();
        let read = write!(stdout, "{message}：")
            .and_then(|()| stdout.flush())
            .and_then(|()| io::stdin().read_line(&mut line));

        match read {
            Ok(0) => {
                self.input_end = Some(ExitCode::SUCCESS);
                None
            }
            Ok(_) => Some(line.trim().to_owned()),
            Err(error) => {
                self.input_error(error);
                None
            }
        }
    }

    fn input_error(&mut self, error: io::Error) {
        eprintln!("命令行输入失败：{error}");
        self.input_end = Some(ExitCode::FAILURE);
    }
}
