import CSUSTKitRust
import Foundation

@MainActor
enum EducationMenu {
    private enum Action {
        case profile, exams, grades, schedule, classrooms, semesterStart
    }

    static func run(_ console: Console, helper: EduHelper, sso: SsoHelper) async {
        do {
            try await sso.loginToEducation()
        } catch {
            Console.report("进入教务系统失败", error)
            return
        }

        let actions: [(Action, String)] = [
            (.profile, "查看个人信息"),
            (.exams, "查看考试安排"),
            (.grades, "查看课程成绩"),
            (.schedule, "查看课程表"),
            (.classrooms, "查看教室课表"),
            (.semesterStart, "查看学期首日"),
        ]
        while let action = console.menu("教务系统", actions) {
            do {
                switch action {
                case .profile:
                    try await profile(helper)
                case .exams:
                    try await exams(console, helper: helper)
                case .grades:
                    try await grades(console, helper: helper)
                case .schedule:
                    try await schedule(console, helper: helper)
                case .classrooms:
                    try await classrooms(console, helper: helper)
                case .semesterStart:
                    try await semesterStart(console, helper: helper)
                }
            } catch {
                Console.report("操作失败", error)
            }
        }
    }

    private static func profile(_ helper: EduHelper) async throws {
        let profile = try await helper.getProfile()
        print("姓名：\(profile.name)")
        print("学号：\(profile.studentId)")
        print("院系：\(profile.department)")
        print("专业：\(profile.major)")
        print("班级：\(profile.className)")
        print("联系电话：\(profile.personalPhone)")
    }

    private static func exams(_ console: Console, helper: EduHelper) async throws {
        let options = try await helper.getAvailableSemestersForExamSchedule()
        guard let semester = chooseSemester(console, options: options) else { return }
        let query = ExamScheduleQuery(academicYearSemester: semester, semesterType: nil)
        let exams = try await helper.getExamSchedule(query: query)
        guard !exams.isEmpty else {
            print("暂无考试安排。")
            return
        }
        print("\n考试安排")
        for (index, exam) in exams.enumerated() {
            print("\(index + 1). \(exam.courseName) [\(exam.courseId)]")
            print("   时间：\(exam.examTime)")
            print("   校区：\(exam.campus) | 考场：\(exam.examRoom) | 座位号：\(exam.seatNumber)")
            print("   教师：\(exam.teacher) | 场次：\(exam.session)")
        }
    }

    private static func grades(_ console: Console, helper: EduHelper) async throws {
        let semesters = try await helper.getAvailableSemestersForCourseGrades()
        let options: [(String?, String)] = [(nil, "全部学期")] + semesters.map { ($0, $0) }
        guard case .some(let semester) = console.menu("选择成绩学期", options) else { return }

        let query = CourseGradeQuery(
            academicYearSemester: semester, courseNature: nil, courseName: "",
            displayMode: .bestGrade, studyMode: .major
        )
        let grades: [CourseGrade]
        do {
            grades = try await helper.getCourseGrades(query: query)
        } catch EduError.CourseGradesRetrievalFailed(let message)
            where message == "课程成绩获取失败: 未查询到数据"
        {
            print("暂无课程成绩。")
            return
        }
        guard !grades.isEmpty else {
            print("暂无课程成绩。")
            return
        }
        guard let grade = console.select("课程成绩（选择课程查看详情）", grades, label: gradeLabel)
        else { return }

        let detail = try await helper.getGradeDetail(url: grade.gradeDetailUrl)
        print("\n\(grade.courseName) 成绩详情（总评 \(detail.totalGrade)）")
        for component in detail.components {
            print("\(component.componentType)：\(component.grade)（占比 \(component.ratio)%）")
        }
    }

    private static func gradeLabel(_ grade: CourseGrade) -> String {
        "\(grade.courseName) [\(grade.courseId)]\n"
            + "   学期：\(grade.semester) | 成绩：\(grade.grade) | 绩点：\(grade.gradePoint) | 学分：\(grade.credit)"
            + " | 考核方式：\(grade.assessmentMethod) | 课程性质：\(grade.courseNature.displayName())"
    }

    private static func schedule(_ console: Console, helper: EduHelper) async throws {
        let options = try await helper.getAvailableSemestersForCourseSchedule()
        guard let semester = chooseSemester(console, options: options) else { return }
        let schedule = try await helper.getCourseSchedule(academicYearSemester: semester)

        if schedule.courses.isEmpty {
            print("暂无课程表数据。")
        } else {
            print("\n课程表")
            for (index, course) in schedule.courses.enumerated() {
                let teacher = ServiceDisplay.value(course.teacher, fallback: "未知教师")
                print("\(index + 1). \(course.courseName) - \(teacher)")
                let sessions = course.sessions.sorted {
                    (weekdayNumber($0.dayOfWeek), $0.startSection, $0.endSection)
                        < (weekdayNumber($1.dayOfWeek), $1.startSection, $1.endSection)
                }
                for session in sessions {
                    let time = sessionLabel(
                        session.dayOfWeek, start: session.startSection, end: session.endSection,
                        weeks: session.weeks
                    )
                    print("   \(time) | 教室：\(session.classroom ?? "未提供")")
                }
            }
        }
        if !schedule.remarks.isEmpty {
            print("备注")
            for remark in schedule.remarks {
                print("- \(remark)")
            }
        }
    }

    private static func classrooms(_ console: Console, helper: EduHelper) async throws {
        let weekdays: [DayOfWeek] = [.monday, .tuesday, .wednesday, .thursday, .friday, .saturday, .sunday]
        guard let campus = ServiceDisplay.campus(console),
            let week = console.number("请输入周次", in: 1...30),
            let weekday = console.select("选择星期", weekdays, label: dayName),
            let section = console.number("请输入大节", in: 1...5)
        else { return }

        let day = weekdayNumber(weekday)
        let start = section * 2 - 1
        let query = ClassroomScheduleQuery(
            campus: campus,
            academicYearSemester: nil,
            weeks: InclusiveRange(start: week, end: week),
            weekdays: InclusiveRange(start: day, end: day),
            sections: InclusiveRange(start: start, end: start + 1)
        )
        let schedule = try await helper.getClassroomSchedule(query: query)
        guard !schedule.isEmpty else {
            print("\(campus.displayName())在所选时段暂无排课记录。")
            return
        }
        print("\n\(campus.displayName())教室课表")
        for session in schedule {
            let time = sessionLabel(
                session.dayOfWeek, start: session.startSection, end: session.endSection,
                weeks: session.weeks
            )
            print("\(session.classroom) | \(time) | \(session.courseLabel)")
        }
    }

    private static func semesterStart(_ console: Console, helper: EduHelper) async throws {
        let options = try await helper.getAvailableSemestersForStartDate()
        guard let semester = chooseSemester(console, options: options) else { return }
        let date = try await helper.getSemesterStartDate(academicYearSemester: semester)
        let dateLabel = String(format: "%04d-%02u-%02u", date.year, date.month, date.day)
        print("学期首日：\(dateLabel)")
    }

    private static func chooseSemester(_ console: Console, options: SemesterOptions) -> String? {
        guard !options.semesters.isEmpty else {
            print("暂无可用学期。")
            return nil
        }
        return console.select("选择学期", options.semesters) { semester in
            semester == options.defaultSemester ? "\(semester)（默认）" : semester
        }
    }

    private static func weekdayNumber(_ day: DayOfWeek) -> Int64 {
        day == .sunday ? 7 : Int64(day.rawValue)
    }

    private static func dayName(_ day: DayOfWeek) -> String {
        switch day {
        case .monday: "星期一"
        case .tuesday: "星期二"
        case .wednesday: "星期三"
        case .thursday: "星期四"
        case .friday: "星期五"
        case .saturday: "星期六"
        case .sunday: "星期日"
        }
    }

    private static func sessionLabel(_ day: DayOfWeek, start: Int64, end: Int64, weeks: [Int64]) -> String {
        let weekList = weeks.map(String.init).joined(separator: ",")
        return "\(dayName(day)) 第\(start)-\(end)节 | 周次：\(weekList)"
    }
}
