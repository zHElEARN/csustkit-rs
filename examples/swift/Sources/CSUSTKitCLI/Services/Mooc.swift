import CSUSTKitRust

@MainActor
enum MoocMenu {
    private enum Action {
        case profile, courses, pending, assignments, exams
    }

    static func run(_ console: Console, helper: MoocHelper, sso: SsoHelper) async {
        do {
            try await sso.loginToMooc()
        } catch {
            Console.report("进入网络课程中心失败", error)
            return
        }

        let actions: [(Action, String)] = [
            (.profile, "查看个人信息"),
            (.courses, "查看课程列表"),
            (.pending, "查看有待交作业的课程"),
            (.assignments, "查看课程作业"),
            (.exams, "查看课程测验"),
        ]
        while let action = console.menu("网络课程中心", actions) {
            do {
                switch action {
                case .profile:
                    let profile = try await helper.getProfile()
                    print("姓名：\(profile.name)")
                    print("上次登录：\(profile.lastLoginTime)")
                    print("在线总时长：\(profile.totalOnlineTime)")
                    print("登录次数：\(profile.loginCount)")
                case .courses:
                    printCourses(try await helper.getCourses(), title: "课程列表", empty: "暂无课程数据。")
                case .pending:
                    printCourses(
                        try await helper.getCoursesWithPendingAssignments(),
                        title: "待提交作业课程",
                        empty: "暂无待完成作业课程。"
                    )
                case .assignments:
                    if let course = try await chooseCourse(console, helper: helper) {
                        printAssignments(
                            try await helper.getCourseAssignments(course: course), course: course)
                    }
                case .exams:
                    if let course = try await chooseCourse(console, helper: helper) {
                        printExams(try await helper.getCourseExams(course: course), course: course)
                    }
                }
            } catch {
                Console.report("操作失败", error)
            }
        }
    }

    private static func chooseCourse(_ console: Console, helper: MoocHelper) async throws -> MoocCourse? {
        let courses = try await helper.getCourses().sorted { $0.name < $1.name }
        if courses.isEmpty {
            print("暂无课程数据。")
            return nil
        }
        return console.select("选择课程", courses) { course in
            let number = ServiceDisplay.value(course.number, fallback: "无编号")
            let teacher = ServiceDisplay.value(course.teacher, fallback: "未知教师")
            return "\(course.name) [\(number)] - \(teacher)"
        }
    }

    private static func printCourses(_ courses: [MoocCourse], title: String, empty: String) {
        guard !courses.isEmpty else {
            print(empty)
            return
        }
        print("\n\(title)")
        for (index, course) in courses.sorted(by: { $0.name < $1.name }).enumerated() {
            let number = ServiceDisplay.value(course.number, fallback: "无编号")
            let teacher = ServiceDisplay.value(course.teacher, fallback: "未知教师")
            let department = ServiceDisplay.value(course.department, fallback: "未知院系")
            print("\(index + 1). \(course.name)")
            print("   编号：\(number) | 教师：\(teacher) | 院系：\(department)")
        }
    }

    private static func printAssignments(_ assignments: [MoocAssignment], course: MoocCourse) {
        guard !assignments.isEmpty else {
            print("\(course.name) 暂无作业数据。")
            return
        }
        print("\n\(course.name) 的作业")
        for (index, assignment) in assignments.enumerated() {
            print("\(index + 1). \(assignment.title)")
            print("   发布人：\(assignment.publisher)")
            print("   开始时间：\(ServiceDisplay.date(assignment.startTime))")
            print("   截止时间：\(ServiceDisplay.date(assignment.deadline))")
            print("   可提交：\(yesNo(assignment.canSubmit)) | 已提交：\(yesNo(assignment.submitStatus))")
        }
    }

    private static func printExams(_ exams: [MoocExam], course: MoocCourse) {
        guard !exams.isEmpty else {
            print("\(course.name) 暂无测验数据。")
            return
        }
        print("\n\(course.name) 的测验")
        for (index, exam) in exams.enumerated() {
            let retakes = exam.allowRetake.map(String.init) ?? "不限制"
            print("\(index + 1). \(exam.title)")
            print("   开始时间：\(exam.startTime)")
            print("   截止时间：\(exam.endTime)")
            print("   限时：\(exam.timeLimit) 分钟 | 可重考次数：\(retakes) | 已交卷：\(yesNo(exam.isSubmitted))")
        }
    }

    private static func yesNo(_ value: Bool) -> String {
        value ? "是" : "否"
    }
}
