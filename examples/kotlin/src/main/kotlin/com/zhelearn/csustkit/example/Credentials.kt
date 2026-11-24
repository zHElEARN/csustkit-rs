package com.zhelearn.csustkit.example

import java.io.File
import java.io.IOException

internal class Credentials {
    private val environment = System.getenv()
    private val fileValues = loadDotenv()

    operator fun get(key: String): String? = environment[key] ?: fileValues[key]

    private fun loadDotenv(): Map<String, String> = try {
        buildMap {
            File(".env").useLines { lines ->
                for (line in lines) {
                    val separator = line.indexOf('=')
                    if (separator < 0) continue
                    val key = line.substring(0, separator).trim()
                    if (key.startsWith("CSUST_") && key !in this) {
                        put(key, line.substring(separator + 1).trim())
                    }
                }
            }
        }
    } catch (_: IOException) {
        emptyMap()
    }
}
