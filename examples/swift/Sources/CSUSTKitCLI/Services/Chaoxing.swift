import CSUSTKitRust

@MainActor
enum ChaoxingMenu {
    private enum Action {
        case profile, assignments
    }

    static func run(_ console: Console, helper: ChaoxingHelper, sso: SsoHelper) async {
        do {
            try await sso.loginToChaoxing()
        } catch {
            Console.report("进入学习通失败", error)
            return
        }

        let actions: [(Action, String)] = [
            (.profile, "查看个人信息"),
            (.assignments, "查看全部作业"),
        ]
        while let action = console.menu("超星学习通", actions) {
            do {
                switch action {
                case .profile:
                    let profile = try await helper.getProfile()
                    print("姓名：\(profile.name)")
                case .assignments:
                    let assignments = try await helper.getAssignments()
                    if assignments.isEmpty { print("暂无作业。") }
                    for assignment in assignments {
                        print(
                            "\(assignment.courseName) | \(assignment.title) | \(assignment.status.displayName())"
                        )
                        if let deadline = assignment.deadline {
                            print("  截止时间：\(ServiceDisplay.date(deadline))")
                        }
                        print("  详情：\(assignment.detailUrl)")
                    }
                }
            } catch {
                Console.report("操作失败", error)
            }
        }
    }
}
