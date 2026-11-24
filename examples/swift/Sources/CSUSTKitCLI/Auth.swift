import CSUSTKitRust
import Foundation

@MainActor
enum Auth {
    static func login(_ console: Console, sso: SsoHelper, session: CsustSession) async -> Bool {
        print("\n=== 统一认证登录 ===")
        repeat {
            guard let username = console.username("请输入统一认证用户名", key: "CSUST_AUTHSERVER_USERNAME"),
                let password = console.password("请输入统一认证密码", key: "CSUST_AUTHSERVER_PASSWORD")
            else { return false }

            do {
                let form = try await sso.getLoginForm()
                var captcha: String?
                if try await sso.checkNeedCaptcha(username: username) {
                    let image = try await sso.getCaptcha()
                    guard let answer = try promptCaptcha(console, image: image) else { return false }
                    captcha = answer
                }
                try await sso.login(loginForm: form, username: username, password: password, captcha: captcha)
                print("统一认证登录成功。")
                return true
            } catch {
                Console.report("统一认证登录失败", error)
            }

            guard console.menu("统一认证登录", [(true, "重试登录")]) == true else { return false }
            session.clearCookies()
        } while true
    }

    static func profile(_ sso: SsoHelper) async {
        do {
            let profile = try await sso.getProfile()
            print("姓名：\(profile.userName)")
            print("账号：\(profile.userAccount)")
            print("院系：\(profile.deptName)")
        } catch {
            Console.report("获取统一认证个人资料失败", error)
        }
    }

    private static func promptCaptcha(_ console: Console, image: Data) throws -> String? {
        let files = FileManager.default
        let file = files.temporaryDirectory.appendingPathComponent(
            "csustkit-captcha-\(UUID().uuidString).jpg")
        try image.write(to: file, options: .withoutOverwriting)
        defer { try? files.removeItem(at: file) }
        print("验证码已保存到 \(file.path)")
        return console.text("请输入验证码")
    }
}
