package com.zhelearn.csustkit.example.services

import com.zhelearn.csustkit.ChaoxingHelper
import com.zhelearn.csustkit.SsoHelper
import com.zhelearn.csustkit.example.Console
import com.zhelearn.csustkit.example.reportError

private enum class ChaoxingAction { PROFILE, ASSIGNMENTS }

internal suspend fun runChaoxing(console: Console, helper: ChaoxingHelper, sso: SsoHelper) {
    try {
        sso.loginToChaoxing()
    } catch (error: Exception) {
        reportError("进入学习通失败", error)
        return
    }

    val actions = listOf(ChaoxingAction.PROFILE to "查看个人信息", ChaoxingAction.ASSIGNMENTS to "查看全部作业")
    while (true) {
        try {
            when (console.menu("超星学习通", actions) ?: return) {
                ChaoxingAction.PROFILE -> println("姓名：${helper.getProfile().name}")
                ChaoxingAction.ASSIGNMENTS -> {
                    val assignments = helper.getAssignments()
                    if (assignments.isEmpty()) println("暂无作业。")
                    for (assignment in assignments) {
                        println("${assignment.courseName} | ${assignment.title} | ${assignment.status.displayName()}")
                        assignment.deadline?.let { println("  截止时间：${displayDate(it)}") }
                        println("  详情：${assignment.detailUrl}")
                    }
                }
            }
        } catch (error: Exception) {
            reportError("操作失败", error)
        }
    }
}
