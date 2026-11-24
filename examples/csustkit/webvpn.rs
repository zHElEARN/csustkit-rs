use csustkit::{webvpn_decrypt_url, webvpn_encrypt_url};
use url::Url;

use super::console::Console;

#[derive(Clone, Copy)]
enum Conversion {
    ToWebVpn,
    ToOriginal,
}

pub fn run_menu(console: &mut Console) {
    while let Some(conversion) = console.menu(
        "WebVPN 链接转换",
        &[
            (Conversion::ToWebVpn, "原始 URL 转 WebVPN URL"),
            (Conversion::ToOriginal, "WebVPN URL 还原原始 URL"),
        ],
        "返回",
    ) {
        convert(console, conversion);
    }
}

fn convert(console: &mut Console, conversion: Conversion) {
    let prompt = match conversion {
        Conversion::ToWebVpn => "请输入原始 URL（0 返回）",
        Conversion::ToOriginal => "请输入 WebVPN URL（0 返回）",
    };
    let Some(input) = console.text(prompt) else {
        return;
    };
    if input == "0" {
        return;
    }
    let url = match Url::parse(&input) {
        Ok(url) => url,
        Err(error) => {
            eprintln!("URL 格式错误：{error}");
            return;
        }
    };
    let result = match conversion {
        Conversion::ToWebVpn => webvpn_encrypt_url(url),
        Conversion::ToOriginal => webvpn_decrypt_url(url),
    };
    match result {
        Ok(url) => println!("{url}"),
        Err(error) => eprintln!("转换失败：{error}"),
    }
}
