package com.zhelearn.csustkit.example

import java.io.IOError
import java.io.IOException

internal class Console {
    private val terminal = System.console()
    private val stdin = if (terminal == null) System.`in`.bufferedReader(Charsets.UTF_8) else null
    private val credentials = Credentials()
    private var inputEnded = false
    var exitCode = 0
        private set

    fun <T> menu(title: String, options: List<Pair<T, String>>, back: String = "返回"): T? =
        select(title, options, back) { it.second }?.first

    fun <T> select(title: String, options: List<T>, back: String = "返回", label: (T) -> String): T? {
        if (inputEnded || options.isEmpty()) return null

        println("\n=== $title ===")
        for ((index, option) in options.withIndex()) {
            println("${index + 1}. ${label(option)}")
        }
        println("0. $back")

        while (true) {
            val input = prompt("请选择") ?: return null
            if (input == "0") return null
            val number = input.toIntOrNull()
            if (number != null && number in 1..options.size) return options[number - 1]
            println("输入无效，请重新选择。")
        }
    }

    fun text(message: String): String? {
        while (true) {
            val input = prompt(message) ?: return null
            if (input.isNotEmpty()) return input
            println("输入不能为空，请重新输入。")
        }
    }

    fun number(message: String, range: LongRange): Long? {
        val label = "$message（${range.first}–${range.last}，0 返回）"
        while (true) {
            val input = prompt(label) ?: return null
            if (input == "0") return null
            val number = input.toLongOrNull()
            if (number != null && number in range) return number
            println("输入无效，请重新输入。")
        }
    }

    fun username(message: String, key: String): String? =
        credentials[key]?.takeIf { it.isNotBlank() } ?: text(message)

    fun password(message: String, key: String): String? {
        if (inputEnded) return null
        credentials[key]?.takeIf { it.isNotEmpty() }?.let { return it }
        if (terminal == null) {
            System.err.println("密码输入需要交互式终端。也可设置 $key。")
            inputEnded = true
            exitCode = 1
            return null
        }

        while (true) {
            val password = read {
                val characters = terminal.readPassword("%s：", message) ?: return@read null
                try {
                    characters.concatToString()
                } finally {
                    characters.fill('\u0000')
                }
            } ?: return null
            if (password.isNotBlank()) return password
            println("输入不能为空，请重新输入。")
        }
    }

    private fun prompt(message: String): String? = read {
        if (terminal != null) {
            terminal.readLine("%s：", message)
        } else {
            print("$message：")
            System.out.flush()
            stdin?.readLine()
        }
    }?.trim()

    private fun read(readLine: () -> String?): String? {
        if (inputEnded) return null
        val input = try {
            readLine()
        } catch (error: IOException) {
            inputFailure(error)
            null
        } catch (error: IOError) {
            inputFailure(error)
            null
        }
        if (input == null) inputEnded = true
        return input
    }

    private fun inputFailure(error: Throwable) {
        reportError("命令行输入失败", error)
        exitCode = 1
    }
}

internal fun reportError(context: String, error: Throwable) {
    System.err.println("$context：${error.message ?: error.javaClass.simpleName}")
}
