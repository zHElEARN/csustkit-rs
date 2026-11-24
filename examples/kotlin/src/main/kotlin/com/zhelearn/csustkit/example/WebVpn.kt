package com.zhelearn.csustkit.example

import com.zhelearn.csustkit.WebVpnException
import com.zhelearn.csustkit.webvpnDecryptUrl
import com.zhelearn.csustkit.webvpnEncryptUrl

private enum class Conversion { TO_WEB_VPN, TO_ORIGINAL }

internal fun runWebVpnMenu(console: Console) {
    val options = listOf(
        Conversion.TO_WEB_VPN to "原始 URL 转 WebVPN URL",
        Conversion.TO_ORIGINAL to "WebVPN URL 还原原始 URL",
    )
    while (true) {
        val conversion = console.menu("WebVPN 链接转换", options) ?: return
        val label = if (conversion == Conversion.TO_WEB_VPN) "原始 URL" else "WebVPN URL"
        val input = console.text("请输入${label}（0 返回）") ?: return
        if (input == "0") continue

        try {
            val url = when (conversion) {
                Conversion.TO_WEB_VPN -> webvpnEncryptUrl(input)
                Conversion.TO_ORIGINAL -> webvpnDecryptUrl(input)
            }
            println(url)
        } catch (error: WebVpnException) {
            reportError("转换失败", error)
        }
    }
}
