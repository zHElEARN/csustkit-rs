use csustkit::{webvpn_decrypt_url, webvpn_encrypt_url};
use url::Url;

use super::console;

pub fn run_menu() {
    loop {
        println!("\n=== WebVPN 工具 ===");
        println!("1. 原始 URL 转 WebVPN URL");
        println!("2. WebVPN URL 还原原始 URL");
        println!("0. 返回上一级");
        let Some(choice) = console::prompt("请选择") else {
            return;
        };
        match choice.as_str() {
            "1" => convert(true),
            "2" => convert(false),
            "0" => return,
            _ => println!("输入无效，请重新选择。"),
        }
    }
}

fn convert(encrypt: bool) {
    let Some(input) = console::prompt_non_empty(if encrypt {
        "请输入原始 URL"
    } else {
        "请输入 WebVPN URL"
    }) else {
        return;
    };
    let Ok(url) = Url::parse(&input) else {
        println!("URL格式错误");
        return;
    };
    let result = if encrypt {
        webvpn_encrypt_url(url)
    } else {
        webvpn_decrypt_url(url)
    };
    match result {
        Ok(url) => println!("\n转换结果:\n{url}"),
        Err(error) => println!("转换失败: {error}"),
    }
}
