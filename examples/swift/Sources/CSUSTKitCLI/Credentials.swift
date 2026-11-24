import Foundation

struct Credentials {
    private let environment = ProcessInfo.processInfo.environment
    private let fileValues = Self.loadDotenv()

    subscript(key: String) -> String? {
        environment[key] ?? fileValues[key]
    }

    private static func loadDotenv() -> [String: String] {
        guard let contents = try? String(contentsOfFile: ".env", encoding: .utf8) else {
            return [:]
        }
        var values: [String: String] = [:]
        for line in contents.split(whereSeparator: \.isNewline) {
            let assignment = line.split(separator: "=", maxSplits: 1, omittingEmptySubsequences: false)
            guard assignment.count == 2 else { continue }
            let key = assignment[0].trimmingCharacters(in: .whitespaces)
            guard key.hasPrefix("CSUST_"), values[key] == nil else { continue }
            values[key] = assignment[1].trimmingCharacters(in: .whitespaces)
        }
        return values
    }
}
