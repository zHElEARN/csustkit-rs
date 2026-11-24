import CSUSTKitRust

@MainActor
enum WebVPNMenu {
    private enum Conversion {
        case toWebVPN, toOriginal
    }

    static func run(_ console: Console) {
        let choices: [(Conversion, String)] = [
            (.toWebVPN, "原始 URL 转 WebVPN URL"),
            (.toOriginal, "WebVPN URL 还原原始 URL"),
        ]
        while let conversion = console.menu("WebVPN 链接转换", choices) {
            let label = conversion == .toWebVPN ? "原始 URL" : "WebVPN URL"
            guard let url = console.text("请输入\(label)（0 返回）"), url != "0" else { continue }
            do {
                switch conversion {
                case .toWebVPN:
                    print(try webvpnEncryptUrl(url: url))
                case .toOriginal:
                    print(try webvpnDecryptUrl(url: url))
                }
            } catch {
                Console.report("转换失败", error)
            }
        }
    }
}
