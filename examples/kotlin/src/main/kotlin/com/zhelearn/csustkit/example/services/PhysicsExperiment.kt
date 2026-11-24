package com.zhelearn.csustkit.example.services

import com.zhelearn.csustkit.PhysicsExperimentHelper
import com.zhelearn.csustkit.example.Console
import com.zhelearn.csustkit.example.reportError

private enum class PhysicsExperimentAction { COURSES, GRADES, LOGOUT }

internal suspend fun runPhysicsExperiment(console: Console, helper: PhysicsExperimentHelper) {
    val username = console.username("请输入大物实验用户名", "CSUST_PHYSICS_EXPERIMENT_USERNAME") ?: return
    val password = console.password("请输入大物实验密码", "CSUST_PHYSICS_EXPERIMENT_PASSWORD") ?: return
    try {
        helper.login(username, password)
    } catch (error: Exception) {
        reportError("登录大物实验失败", error)
        return
    }

    val actions = listOf(
        PhysicsExperimentAction.COURSES to "查看实验课表",
        PhysicsExperimentAction.GRADES to "查看实验成绩",
        PhysicsExperimentAction.LOGOUT to "登出大物实验",
    )
    while (true) {
        val action = console.menu("大物实验", actions) ?: return
        try {
            when (action) {
                PhysicsExperimentAction.COURSES -> {
                    val courses = helper.getCourses()
                    if (courses.isEmpty()) println("暂无实验排课。")
                    for (course in courses) {
                        val start = displayDate(course.startTime)
                        val end = displayDate(course.endTime)
                        println("${course.name} | ${course.teacher} | ${course.location} | $start 至 $end")
                    }
                }
                PhysicsExperimentAction.GRADES -> {
                    val grades = helper.getCourseGrades()
                    if (grades.isEmpty()) println("暂无实验成绩。")
                    for (grade in grades) {
                        println("${grade.courseName} | ${grade.itemName} | 总成绩 ${grade.totalGrade}")
                    }
                }
                PhysicsExperimentAction.LOGOUT -> {
                    helper.logout()
                    println("已登出大物实验。")
                    return
                }
            }
        } catch (error: Exception) {
            reportError(if (action == PhysicsExperimentAction.LOGOUT) "登出大物实验失败" else "操作失败", error)
        }
    }
}
