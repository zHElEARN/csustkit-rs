package com.zhelearn.csustkit.example.services

import com.zhelearn.csustkit.ClassroomScheduleQuery
import com.zhelearn.csustkit.CourseGrade
import com.zhelearn.csustkit.CourseGradeQuery
import com.zhelearn.csustkit.DayOfWeek
import com.zhelearn.csustkit.DisplayMode
import com.zhelearn.csustkit.EduException
import com.zhelearn.csustkit.EduHelper
import com.zhelearn.csustkit.ExamScheduleQuery
import com.zhelearn.csustkit.InclusiveRange
import com.zhelearn.csustkit.ScheduleSession
import com.zhelearn.csustkit.SemesterOptions
import com.zhelearn.csustkit.SsoHelper
import com.zhelearn.csustkit.StudyMode
import com.zhelearn.csustkit.example.Console
import com.zhelearn.csustkit.example.reportError

private enum class EducationAction { PROFILE, EXAMS, GRADES, SCHEDULE, CLASSROOMS, SEMESTER_START }

internal suspend fun runEducation(console: Console, helper: EduHelper, sso: SsoHelper) {
    try {
        sso.loginToEducation()
    } catch (error: Exception) {
        reportError("进入教务系统失败", error)
        return
    }

    val actions = listOf(
        EducationAction.PROFILE to "查看个人信息",
        EducationAction.EXAMS to "查看考试安排",
        EducationAction.GRADES to "查看课程成绩",
        EducationAction.SCHEDULE to "查看课程表",
        EducationAction.CLASSROOMS to "查看教室课表",
        EducationAction.SEMESTER_START to "查看学期首日",
    )
    while (true) {
        try {
            when (console.menu("教务系统", actions) ?: return) {
                EducationAction.PROFILE -> profile(helper)
                EducationAction.EXAMS -> exams(console, helper)
                EducationAction.GRADES -> grades(console, helper)
                EducationAction.SCHEDULE -> schedule(console, helper)
                EducationAction.CLASSROOMS -> classrooms(console, helper)
                EducationAction.SEMESTER_START -> semesterStart(console, helper)
            }
        } catch (error: Exception) {
            reportError("操作失败", error)
        }
    }
}

private suspend fun profile(helper: EduHelper) {
    val profile = helper.getProfile()
    println("姓名：${profile.name}")
    println("学号：${profile.studentId}")
    println("院系：${profile.department}")
    println("专业：${profile.major}")
    println("班级：${profile.className}")
    println("联系电话：${profile.personalPhone}")
}

private suspend fun exams(console: Console, helper: EduHelper) {
    val options = helper.getAvailableSemestersForExamSchedule()
    val semester = chooseSemester(console, options) ?: return
    val query = ExamScheduleQuery(academicYearSemester = semester, semesterType = null)
    val exams = helper.getExamSchedule(query)
    if (exams.isEmpty()) {
        println("暂无考试安排。")
        return
    }

    println("\n考试安排")
    for ((index, exam) in exams.withIndex()) {
        println("${index + 1}. ${exam.courseName} [${exam.courseId}]")
        println("   时间：${exam.examTime}")
        println("   校区：${exam.campus} | 考场：${exam.examRoom} | 座位号：${exam.seatNumber}")
        println("   教师：${exam.teacher} | 场次：${exam.session}")
    }
}

private suspend fun grades(console: Console, helper: EduHelper) {
    val semesters = helper.getAvailableSemestersForCourseGrades()
    val options = listOf(null to "全部学期") + semesters.map { it to it }
    val (semester) = console.select("选择成绩学期", options) { it.second } ?: return

    val query = CourseGradeQuery(
        academicYearSemester = semester,
        courseNature = null,
        courseName = "",
        displayMode = DisplayMode.BEST_GRADE,
        studyMode = StudyMode.MAJOR,
    )
    val grades = try {
        helper.getCourseGrades(query)
    } catch (error: EduException.CourseGradesRetrievalFailed) {
        if (error.message != "课程成绩获取失败: 未查询到数据") throw error
        emptyList()
    }
    if (grades.isEmpty()) {
        println("暂无课程成绩。")
        return
    }

    val grade = console.select("课程成绩（选择课程查看详情）", grades, label = ::gradeLabel) ?: return
    val detail = helper.getGradeDetail(grade.gradeDetailUrl)
    println("\n${grade.courseName} 成绩详情（总评 ${detail.totalGrade}）")
    for (component in detail.components) {
        println("${component.componentType}：${component.grade}（占比 ${component.ratio}%）")
    }
}

private fun gradeLabel(grade: CourseGrade): String =
    "${grade.courseName} [${grade.courseId}]\n" +
        "   学期：${grade.semester} | 成绩：${grade.grade} | 绩点：${grade.gradePoint} | 学分：${grade.credit}" +
        " | 考核方式：${grade.assessmentMethod} | 课程性质：${grade.courseNature.displayName()}"

private suspend fun schedule(console: Console, helper: EduHelper) {
    val options = helper.getAvailableSemestersForCourseSchedule()
    val semester = chooseSemester(console, options) ?: return
    val schedule = helper.getCourseSchedule(semester)

    if (schedule.courses.isEmpty()) {
        println("暂无课程表数据。")
    } else {
        println("\n课程表")
        for ((index, course) in schedule.courses.withIndex()) {
            val teacher = course.teacher.takeUnless { it.isNullOrEmpty() } ?: "未知教师"
            println("${index + 1}. ${course.courseName} - $teacher")
            val sessions = course.sessions.sortedWith(
                compareBy<ScheduleSession>({ weekdayNumber(it.dayOfWeek) }, { it.startSection }, { it.endSection }),
            )
            for (session in sessions) {
                val time = sessionLabel(session.dayOfWeek, session.startSection, session.endSection, session.weeks)
                println("   $time | 教室：${session.classroom ?: "未提供"}")
            }
        }
    }
    if (schedule.remarks.isNotEmpty()) {
        println("备注")
        for (remark in schedule.remarks) println("- $remark")
    }
}

private suspend fun classrooms(console: Console, helper: EduHelper) {
    val weekdays = listOf(
        DayOfWeek.MONDAY, DayOfWeek.TUESDAY, DayOfWeek.WEDNESDAY, DayOfWeek.THURSDAY,
        DayOfWeek.FRIDAY, DayOfWeek.SATURDAY, DayOfWeek.SUNDAY,
    )
    val campus = chooseCampus(console) ?: return
    val week = console.number("请输入周次", 1L..30L) ?: return
    val weekday = console.select("选择星期", weekdays, label = ::dayName) ?: return
    val section = console.number("请输入大节", 1L..5L) ?: return

    val day = weekdayNumber(weekday)
    val start = section * 2 - 1
    val query = ClassroomScheduleQuery(
        campus = campus,
        academicYearSemester = null,
        weeks = InclusiveRange(start = week, end = week),
        weekdays = InclusiveRange(start = day, end = day),
        sections = InclusiveRange(start = start, end = start + 1),
    )
    val schedule = helper.getClassroomSchedule(query)
    if (schedule.isEmpty()) {
        println("${campus.displayName()}在所选时段暂无排课记录。")
        return
    }

    println("\n${campus.displayName()}教室课表")
    for (session in schedule) {
        val time = sessionLabel(session.dayOfWeek, session.startSection, session.endSection, session.weeks)
        println("${session.classroom} | $time | ${session.courseLabel}")
    }
}

private suspend fun semesterStart(console: Console, helper: EduHelper) {
    val options = helper.getAvailableSemestersForStartDate()
    val semester = chooseSemester(console, options) ?: return
    val date = helper.getSemesterStartDate(semester)
    val month = date.month.toString().padStart(2, '0')
    val day = date.day.toString().padStart(2, '0')
    println("学期首日：${date.year}-$month-$day")
}

private fun chooseSemester(console: Console, options: SemesterOptions): String? {
    if (options.semesters.isEmpty()) {
        println("暂无可用学期。")
        return null
    }
    return console.select("选择学期", options.semesters) { semester ->
        if (semester == options.defaultSemester) "$semester（默认）" else semester
    }
}

private fun weekdayNumber(day: DayOfWeek): Long = if (day == DayOfWeek.SUNDAY) 7 else day.value.toLong()

private fun dayName(day: DayOfWeek): String = when (day) {
    DayOfWeek.MONDAY -> "星期一"
    DayOfWeek.TUESDAY -> "星期二"
    DayOfWeek.WEDNESDAY -> "星期三"
    DayOfWeek.THURSDAY -> "星期四"
    DayOfWeek.FRIDAY -> "星期五"
    DayOfWeek.SATURDAY -> "星期六"
    DayOfWeek.SUNDAY -> "星期日"
}

private fun sessionLabel(day: DayOfWeek, start: Long, end: Long, weeks: List<Long>): String =
    "${dayName(day)} 第$start-${end}节 | 周次：${weeks.joinToString(",")}"
