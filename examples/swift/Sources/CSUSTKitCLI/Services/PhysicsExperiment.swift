import CSUSTKitRust

@MainActor
enum PhysicsExperimentMenu {
    private enum Action {
        case courses, grades, logout
    }

    static func run(_ console: Console, helper: PhysicsExperimentHelper) async {
        guard let username = console.username("请输入大物实验用户名", key: "CSUST_PHYSICS_EXPERIMENT_USERNAME"),
            let password = console.password("请输入大物实验密码", key: "CSUST_PHYSICS_EXPERIMENT_PASSWORD")
        else { return }
        do {
            try await helper.login(username: username, password: password)
        } catch {
            Console.report("登录大物实验失败", error)
            return
        }

        let actions: [(Action, String)] = [
            (.courses, "查看实验课表"),
            (.grades, "查看实验成绩"),
            (.logout, "登出大物实验"),
        ]
        while let action = console.menu("大物实验", actions) {
            do {
                switch action {
                case .courses:
                    let courses = try await helper.getCourses()
                    if courses.isEmpty { print("暂无实验排课。") }
                    for course in courses {
                        let start = ServiceDisplay.date(course.startTime)
                        let end = ServiceDisplay.date(course.endTime)
                        print("\(course.name) | \(course.teacher) | \(course.location) | \(start) 至 \(end)")
                    }
                case .grades:
                    let grades = try await helper.getCourseGrades()
                    if grades.isEmpty { print("暂无实验成绩。") }
                    for grade in grades {
                        print("\(grade.courseName) | \(grade.itemName) | 总成绩 \(grade.totalGrade)")
                    }
                case .logout:
                    try await helper.logout()
                    print("已登出大物实验。")
                    return
                }
            } catch {
                Console.report(action == .logout ? "登出大物实验失败" : "操作失败", error)
            }
        }
    }
}
