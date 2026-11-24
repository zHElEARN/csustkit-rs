import Darwin
import Foundation

@MainActor
final class Console {
    private let credentials = Credentials()
    private var inputEnded = false
    private(set) var exitCode = EXIT_SUCCESS

    func menu<Option>(
        _ title: String, _ choices: [(Option, String)], back: String = "返回"
    ) -> Option? {
        select(title, choices, back: back, label: { $0.1 })?.0
    }

    func select<Item>(
        _ title: String, _ items: [Item], back: String = "返回", label: (Item) -> String
    ) -> Item? {
        guard !inputEnded, !items.isEmpty else { return nil }

        print("\n=== \(title) ===")
        for (index, item) in items.enumerated() {
            print("\(index + 1). \(label(item))")
        }
        print("0. \(back)")

        while let input = prompt("请选择") {
            if input == "0" { return nil }
            if let number = Int(input), (1...items.count).contains(number) {
                return items[number - 1]
            }
            print("输入无效，请重新选择。")
        }
        return nil
    }

    func text(_ message: String) -> String? {
        while let input = prompt(message) {
            if !input.isEmpty { return input }
            print("输入不能为空，请重新输入。")
        }
        return nil
    }

    func number(_ message: String, in range: ClosedRange<Int64>) -> Int64? {
        let label = "\(message)（\(range.lowerBound)–\(range.upperBound)，0 返回）"
        while let input = prompt(label) {
            if input == "0" { return nil }
            if let number = Int64(input), range.contains(number) { return number }
            print("输入无效，请重新输入。")
        }
        return nil
    }

    func username(_ message: String, key: String) -> String? {
        if let value = credentials[key], !value.trimmingCharacters(in: .whitespaces).isEmpty {
            return value
        }
        return text(message)
    }

    func password(_ message: String, key: String) -> String? {
        guard !inputEnded else { return nil }
        if let value = credentials[key], !value.isEmpty { return value }

        guard isatty(STDIN_FILENO) == 1 else {
            inputEnded = true
            exitCode = EXIT_FAILURE
            Self.writeError("密码输入需要交互式终端。也可设置 \(key)。")
            return nil
        }

        // readpassphrase restores terminal echo even when input is interrupted.
        var buffer = [CChar](repeating: 0, count: 4096)
        defer { buffer.withUnsafeMutableBytes { _ = $0.initializeMemory(as: UInt8.self, repeating: 0) } }
        while true {
            fflush(nil)
            let password = buffer.withUnsafeMutableBufferPointer { bytes -> String? in
                guard let start = bytes.baseAddress,
                    readpassphrase("\(message)（留空退出）：", start, bytes.count, RPP_REQUIRE_TTY) != nil
                else { return nil }
                return String(validatingCString: start)
            }
            guard let password else {
                inputEnded = true
                exitCode = EXIT_FAILURE
                Self.writeError("无法读取密码。")
                return nil
            }
            if password.isEmpty {
                inputEnded = true
                return nil
            }
            if password.utf8.count == buffer.count - 1 {
                print("密码过长，请重新输入。")
            } else if !password.trimmingCharacters(in: .whitespaces).isEmpty {
                return password
            } else {
                print("输入不能为空，请重新输入。")
            }
        }
    }

    static func report(_ context: String, _ error: any Error) {
        writeError("\(context)：\(error.localizedDescription)")
    }

    private static func writeError(_ message: String) {
        FileHandle.standardError.write(Data("\(message)\n".utf8))
    }

    private func prompt(_ message: String) -> String? {
        guard !inputEnded else { return nil }
        print("\(message)：", terminator: "")
        fflush(nil)
        guard let input = readLine() else {
            inputEnded = true
            return nil
        }
        return input.trimmingCharacters(in: .whitespacesAndNewlines)
    }
}
