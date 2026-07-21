use std::io::{self, Write};

pub fn prompt(message: &str) -> Option<String> {
    print!("{message}：");
    if let Err(error) = io::stdout().flush() {
        exit_on_input_error(error);
    }
    let mut input = String::new();
    match io::stdin().read_line(&mut input) {
        Ok(0) => None,
        Ok(_) => Some(input.trim().to_owned()),
        Err(error) => exit_on_input_error(error),
    }
}

pub fn prompt_non_empty(message: &str) -> Option<String> {
    loop {
        let input = prompt(message)?;
        if !input.is_empty() {
            return Some(input);
        }
        println!("输入不能为空，请重新输入。\n");
    }
}

pub fn prompt_password(message: &str) -> Option<String> {
    loop {
        match rpassword::prompt_password(format!("{message}：")) {
            Ok(input) if !input.trim().is_empty() => return Some(input),
            Ok(_) => println!("输入不能为空，请重新输入。\n"),
            Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => return None,
            Err(error) => exit_on_input_error(error),
        }
    }
}

pub fn prompt_int(message: &str, min: i64, max: i64) -> Option<i64> {
    loop {
        let input = prompt(message)?;
        match input.parse::<i64>() {
            Ok(value) if (min..=max).contains(&value) => return Some(value),
            _ => println!("输入无效，请输入 {min}-{max} 之间的整数。"),
        }
    }
}

pub fn select<'a, T>(title: &str, items: &'a [T], display: impl Fn(&T) -> String) -> Option<&'a T> {
    if items.is_empty() {
        return None;
    }
    loop {
        println!("\n{title}");
        for (index, item) in items.iter().enumerate() {
            println!("{}. {}", index + 1, display(item));
        }
        println!("0. 返回");
        let input = prompt("请选择")?;
        if input == "0" {
            return None;
        }
        if let Ok(index) = input.parse::<usize>() {
            if let Some(item) = items.get(index.saturating_sub(1)) {
                return Some(item);
            }
        }
        println!("输入无效，请重新选择。");
    }
}

pub fn select_value<T: Copy>(title: &str, options: &[(T, &str)]) -> Option<T> {
    loop {
        println!("\n{title}");
        for (index, (_, label)) in options.iter().enumerate() {
            println!("{}. {label}", index + 1);
        }
        println!("0. 返回");
        let input = prompt("请选择")?;
        if input == "0" {
            return None;
        }
        if let Ok(index) = input.parse::<usize>() {
            if let Some((value, _)) = options.get(index.saturating_sub(1)) {
                return Some(*value);
            }
        }
        println!("输入无效，请重新选择。");
    }
}

fn exit_on_input_error(error: io::Error) -> ! {
    eprintln!("读取输入失败: {error}");
    std::process::exit(1);
}
