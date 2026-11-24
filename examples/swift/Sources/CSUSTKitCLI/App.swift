import CSUSTKitRust
import Darwin

@main
@MainActor
enum CSUSTKitCLI {
    private enum Entry {
        case campusServices, physicsExperiment, webVPN
    }

    private enum Service {
        case mooc, education, campusCard, chaoxing, physicsExperiment, profile
    }

    static func main() async {
        let console = Console()
        let entries: [(Entry, String)] = [
            (.campusServices, "校园服务（统一认证）"),
            (.physicsExperiment, "大物实验（校园网直连）"),
            (.webVPN, "WebVPN 链接转换"),
        ]
        while let entry = console.menu("CSUSTKit", entries, back: "退出") {
            switch entry {
            case .campusServices:
                await campusServices(console)
            case .physicsExperiment:
                if let session = newSession() {
                    let helper = PhysicsExperimentHelper(mode: .direct, session: session)
                    await PhysicsExperimentMenu.run(console, helper: helper)
                }
            case .webVPN:
                WebVPNMenu.run(console)
            }
        }

        print("程序已退出。")
        if console.exitCode != EXIT_SUCCESS {
            exit(console.exitCode)
        }
    }

    private static func campusServices(_ console: Console) async {
        let modes: [(ConnectionMode, String)] = [(.direct, "校园网直连"), (.webVpn, "WebVPN")]
        guard let mode = console.menu("选择网络模式", modes),
            let session = newSession()
        else { return }

        let sso = SsoHelper(mode: mode, session: session)
        guard await Auth.login(console, sso: sso, session: session) else { return }

        let services: [(Service, String)] = [
            (.mooc, "网络课程中心"),
            (.education, "教务系统"),
            (.campusCard, "校园卡"),
            (.chaoxing, "超星学习通"),
            (.physicsExperiment, "大物实验（独立账号）"),
            (.profile, "统一认证个人资料"),
        ]
        let title = mode == .direct ? "校园服务（校园网直连）" : "校园服务（WebVPN）"
        while let service = console.menu(title, services, back: "结束本次会话") {
            // Each helper uses the cookie jar from this SSO session.
            switch service {
            case .mooc:
                let helper = MoocHelper(mode: mode, session: session)
                await MoocMenu.run(console, helper: helper, sso: sso)
            case .education:
                let helper = EduHelper(mode: mode, session: session)
                await EducationMenu.run(console, helper: helper, sso: sso)
            case .campusCard:
                let helper = CampusCardHelper(mode: mode, session: session)
                await CampusCardMenu.run(console, helper: helper, sso: sso)
            case .chaoxing:
                let helper = ChaoxingHelper(mode: mode, session: session)
                await ChaoxingMenu.run(console, helper: helper, sso: sso)
            case .physicsExperiment:
                let helper = PhysicsExperimentHelper(mode: mode, session: session)
                await PhysicsExperimentMenu.run(console, helper: helper)
            case .profile:
                await Auth.profile(sso)
            }
        }
    }

    private static func newSession() -> CsustSession? {
        do {
            return try CsustSession()
        } catch {
            Console.report("无法创建会话", error)
            return nil
        }
    }
}
