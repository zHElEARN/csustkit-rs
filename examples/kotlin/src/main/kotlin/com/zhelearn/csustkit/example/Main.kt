package com.zhelearn.csustkit.example

import com.zhelearn.csustkit.CampusCardHelper
import com.zhelearn.csustkit.ChaoxingHelper
import com.zhelearn.csustkit.ConnectionMode
import com.zhelearn.csustkit.CsustSession
import com.zhelearn.csustkit.EduHelper
import com.zhelearn.csustkit.MoocHelper
import com.zhelearn.csustkit.PhysicsExperimentHelper
import com.zhelearn.csustkit.SessionInitException
import com.zhelearn.csustkit.SsoHelper
import com.zhelearn.csustkit.example.services.runCampusCard
import com.zhelearn.csustkit.example.services.runChaoxing
import com.zhelearn.csustkit.example.services.runEducation
import com.zhelearn.csustkit.example.services.runMooc
import com.zhelearn.csustkit.example.services.runPhysicsExperiment
import kotlinx.coroutines.runBlocking
import kotlin.system.exitProcess

private enum class Entry { CAMPUS_SERVICES, PHYSICS_EXPERIMENT, WEB_VPN }

private enum class Service { MOOC, EDUCATION, CAMPUS_CARD, CHAOXING, PHYSICS_EXPERIMENT, PROFILE }

fun main() {
    val console = Console()
    try {
        runBlocking { runMenu(console) }
    } catch (error: LinkageError) {
        reportError("加载 Rust 动态库失败", error)
        exitProcess(1)
    }
    println("程序已退出。")
    if (console.exitCode != 0) exitProcess(console.exitCode)
}

private suspend fun runMenu(console: Console) {
    val entries = listOf(
        Entry.CAMPUS_SERVICES to "校园服务（统一认证）",
        Entry.PHYSICS_EXPERIMENT to "大物实验（校园网直连）",
        Entry.WEB_VPN to "WebVPN 链接转换",
    )
    while (true) {
        when (console.menu("CSUSTKit", entries, back = "退出") ?: return) {
            Entry.CAMPUS_SERVICES -> campusServices(console)
            Entry.PHYSICS_EXPERIMENT -> newSession()?.use { session ->
                PhysicsExperimentHelper(ConnectionMode.DIRECT, session).use { helper ->
                    runPhysicsExperiment(console, helper)
                }
            }
            Entry.WEB_VPN -> runWebVpnMenu(console)
        }
    }
}

private suspend fun campusServices(console: Console) {
    val modes = listOf(ConnectionMode.DIRECT to "校园网直连", ConnectionMode.WEB_VPN to "WebVPN")
    val mode = console.menu("选择网络模式", modes) ?: return
    val session = newSession() ?: return

    session.use {
        SsoHelper(mode, session).use { sso ->
            if (login(console, sso, session)) {
                serviceMenu(console, mode, session, sso)
            }
        }
    }
}

private suspend fun serviceMenu(console: Console, mode: ConnectionMode, session: CsustSession, sso: SsoHelper) {
    val services = listOf(
        Service.MOOC to "网络课程中心",
        Service.EDUCATION to "教务系统",
        Service.CAMPUS_CARD to "校园卡",
        Service.CHAOXING to "超星学习通",
        Service.PHYSICS_EXPERIMENT to "大物实验（独立账号）",
        Service.PROFILE to "统一认证个人资料",
    )
    val title = when (mode) {
        ConnectionMode.DIRECT -> "校园服务（校园网直连）"
        ConnectionMode.WEB_VPN -> "校园服务（WebVPN）"
    }
    while (true) {
        // Each helper uses the cookie jar from this SSO session.
        when (console.menu(title, services, back = "结束本次会话") ?: return) {
            Service.MOOC -> MoocHelper(mode, session).use { runMooc(console, it, sso) }
            Service.EDUCATION -> EduHelper(mode, session).use { runEducation(console, it, sso) }
            Service.CAMPUS_CARD -> CampusCardHelper(mode, session).use { runCampusCard(console, it, sso) }
            Service.CHAOXING -> ChaoxingHelper(mode, session).use { runChaoxing(console, it, sso) }
            Service.PHYSICS_EXPERIMENT -> PhysicsExperimentHelper(mode, session).use { runPhysicsExperiment(console, it) }
            Service.PROFILE -> showProfile(sso)
        }
    }
}

private fun newSession(): CsustSession? = try {
    CsustSession()
} catch (error: SessionInitException) {
    reportError("无法创建会话", error)
    null
}
