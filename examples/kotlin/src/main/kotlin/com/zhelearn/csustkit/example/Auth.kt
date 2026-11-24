package com.zhelearn.csustkit.example

import com.zhelearn.csustkit.CsustSession
import com.zhelearn.csustkit.SsoHelper
import java.io.IOException
import java.nio.file.Files

internal suspend fun login(console: Console, sso: SsoHelper, session: CsustSession): Boolean {
    println("\n=== 统一认证登录 ===")
    while (true) {
        val username = console.username("请输入统一认证用户名", "CSUST_AUTHSERVER_USERNAME") ?: return false
        val password = console.password("请输入统一认证密码", "CSUST_AUTHSERVER_PASSWORD") ?: return false

        try {
            val form = sso.getLoginForm()
            val captcha = if (sso.checkNeedCaptcha(username)) {
                promptCaptcha(console, sso.getCaptcha()) ?: return false
            } else {
                null
            }
            sso.login(form, username, password, captcha)
            println("统一认证登录成功。")
            return true
        } catch (error: Exception) {
            reportError("统一认证登录失败", error)
        }

        console.menu("统一认证登录", listOf(Unit to "重试登录")) ?: return false
        session.clearCookies()
    }
}

internal suspend fun showProfile(sso: SsoHelper) {
    try {
        val profile = sso.getProfile()
        println("姓名：${profile.userName}")
        println("账号：${profile.userAccount}")
        println("院系：${profile.deptName}")
    } catch (error: Exception) {
        reportError("获取统一认证个人资料失败", error)
    }
}

private fun promptCaptcha(console: Console, image: ByteArray): String? {
    val file = Files.createTempFile("csustkit-captcha-", ".jpg")
    try {
        Files.write(file, image)
        println("验证码已保存到 ${file.toAbsolutePath()}")
        return console.text("请输入验证码")
    } finally {
        try {
            Files.deleteIfExists(file)
        } catch (_: IOException) {
            file.toFile().deleteOnExit()
        }
    }
}
