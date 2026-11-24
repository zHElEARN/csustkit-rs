package com.zhelearn.csustkit.example.services

import com.zhelearn.csustkit.CampusCardHelper
import com.zhelearn.csustkit.SsoHelper
import com.zhelearn.csustkit.example.Console
import com.zhelearn.csustkit.example.reportError
import java.util.Locale

private enum class CampusCardAction { PROFILE, ELECTRICITY, LOGOUT }

internal suspend fun runCampusCard(console: Console, helper: CampusCardHelper, sso: SsoHelper) {
    try {
        val ticket = sso.loginToCampusCard()
        helper.syncToken(ticket)
    } catch (error: Exception) {
        reportError("进入校园卡系统失败", error)
        return
    }

    val actions = listOf(
        CampusCardAction.PROFILE to "查看个人信息",
        CampusCardAction.ELECTRICITY to "查询宿舍电量",
        CampusCardAction.LOGOUT to "登出校园卡",
    )
    while (true) {
        val action = console.menu("校园卡", actions) ?: return
        try {
            when (action) {
                CampusCardAction.PROFILE -> {
                    val profile = helper.getProfile()
                    println("姓名：${profile.name}")
                    println("学号：${profile.sno}")
                    println("院系：${profile.departmentName}")
                    println("宿舍班级：${profile.className}")
                }
                CampusCardAction.ELECTRICITY -> electricity(console, helper)
                CampusCardAction.LOGOUT -> {
                    helper.logout()
                    println("已登出校园卡。")
                    return
                }
            }
        } catch (error: Exception) {
            reportError(if (action == CampusCardAction.LOGOUT) "登出校园卡失败" else "操作失败", error)
        }
    }
}

private suspend fun electricity(console: Console, helper: CampusCardHelper) {
    val campus = chooseCampus(console) ?: return

    val buildings = helper.getBuildings(campus).sortedBy { it.name }
    if (buildings.isEmpty()) {
        println("${campus.displayName()}楼栋列表为空。")
        return
    }
    val building = console.select("选择楼栋", buildings) { it.name } ?: return

    val rooms = helper.getRooms(building).sortedBy { it.name }
    if (rooms.isEmpty()) {
        println("${building.name}宿舍列表为空。")
        return
    }
    val room = console.select("选择宿舍", rooms) { it.name } ?: return

    val amount = String.format(Locale.ROOT, "%.2f", helper.getElectricity(room))
    println("${campus.displayName()} | ${building.name} | ${room.name} | 剩余电量 $amount 度")
}
