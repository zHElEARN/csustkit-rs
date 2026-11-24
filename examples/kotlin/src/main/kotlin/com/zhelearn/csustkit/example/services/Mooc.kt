package com.zhelearn.csustkit.example.services

import com.zhelearn.csustkit.MoocAssignment
import com.zhelearn.csustkit.MoocCourse
import com.zhelearn.csustkit.MoocExam
import com.zhelearn.csustkit.MoocHelper
import com.zhelearn.csustkit.SsoHelper
import com.zhelearn.csustkit.example.Console
import com.zhelearn.csustkit.example.reportError

private enum class MoocAction { PROFILE, COURSES, PENDING, ASSIGNMENTS, EXAMS }

internal suspend fun runMooc(console: Console, helper: MoocHelper, sso: SsoHelper) {
    try {
        sso.loginToMooc()
    } catch (error: Exception) {
        reportError("进入网络课程中心失败", error)
        return
    }

    val actions = listOf(
        MoocAction.PROFILE to "查看个人信息",
        MoocAction.COURSES to "查看课程列表",
        MoocAction.PENDING to "查看有待交作业的课程",
        MoocAction.ASSIGNMENTS to "查看课程作业",
        MoocAction.EXAMS to "查看课程测验",
    )
    while (true) {
        val action = console.menu("网络课程中心", actions) ?: return
        try {
            when (action) {
                MoocAction.PROFILE -> {
                    val profile = helper.getProfile()
                    println("姓名：${profile.name}")
                    println("上次登录：${profile.lastLoginTime}")
                    println("在线总时长：${profile.totalOnlineTime}")
                    println("登录次数：${profile.loginCount}")
                }
                MoocAction.COURSES -> printCourses(helper.getCourses(), "课程列表", "暂无课程数据。")
                MoocAction.PENDING -> printCourses(
                    helper.getCoursesWithPendingAssignments(),
                    "待提交作业课程",
                    "暂无待完成作业课程。",
                )
                MoocAction.ASSIGNMENTS -> {
                    val course = chooseCourse(console, helper) ?: continue
                    printAssignments(helper.getCourseAssignments(course), course)
                }
                MoocAction.EXAMS -> {
                    val course = chooseCourse(console, helper) ?: continue
                    printExams(helper.getCourseExams(course), course)
                }
            }
        } catch (error: Exception) {
            reportError("操作失败", error)
        }
    }
}

private suspend fun chooseCourse(console: Console, helper: MoocHelper): MoocCourse? {
    val courses = helper.getCourses().sortedBy { it.name }
    if (courses.isEmpty()) {
        println("暂无课程数据。")
        return null
    }
    return console.select("选择课程", courses) { course ->
        val number = course.number.takeUnless { it.isNullOrEmpty() } ?: "无编号"
        val teacher = course.teacher.takeUnless { it.isNullOrEmpty() } ?: "未知教师"
        "${course.name} [$number] - $teacher"
    }
}

private fun printCourses(courses: List<MoocCourse>, title: String, empty: String) {
    if (courses.isEmpty()) {
        println(empty)
        return
    }
    println("\n$title")
    for ((index, course) in courses.sortedBy { it.name }.withIndex()) {
        val number = course.number.takeUnless { it.isNullOrEmpty() } ?: "无编号"
        val teacher = course.teacher.takeUnless { it.isNullOrEmpty() } ?: "未知教师"
        val department = course.department.takeUnless { it.isNullOrEmpty() } ?: "未知院系"
        println("${index + 1}. ${course.name}")
        println("   编号：$number | 教师：$teacher | 院系：$department")
    }
}

private fun printAssignments(assignments: List<MoocAssignment>, course: MoocCourse) {
    if (assignments.isEmpty()) {
        println("${course.name} 暂无作业数据。")
        return
    }
    println("\n${course.name} 的作业")
    for ((index, assignment) in assignments.withIndex()) {
        println("${index + 1}. ${assignment.title}")
        println("   发布人：${assignment.publisher}")
        println("   开始时间：${displayDate(assignment.startTime)}")
        println("   截止时间：${displayDate(assignment.deadline)}")
        println("   可提交：${yesNo(assignment.canSubmit)} | 已提交：${yesNo(assignment.submitStatus)}")
    }
}

private fun printExams(exams: List<MoocExam>, course: MoocCourse) {
    if (exams.isEmpty()) {
        println("${course.name} 暂无测验数据。")
        return
    }
    println("\n${course.name} 的测验")
    for ((index, exam) in exams.withIndex()) {
        val retakes = exam.allowRetake?.toString() ?: "不限制"
        println("${index + 1}. ${exam.title}")
        println("   开始时间：${exam.startTime}")
        println("   截止时间：${exam.endTime}")
        println("   限时：${exam.timeLimit} 分钟 | 可重考次数：$retakes | 已交卷：${yesNo(exam.isSubmitted)}")
    }
}

private fun yesNo(value: Boolean): String = if (value) "是" else "否"
