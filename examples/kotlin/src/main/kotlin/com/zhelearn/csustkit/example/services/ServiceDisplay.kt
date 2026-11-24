package com.zhelearn.csustkit.example.services

import com.zhelearn.csustkit.Campus
import com.zhelearn.csustkit.example.Console
import java.time.Instant
import java.time.ZoneId
import java.time.format.DateTimeFormatter
import java.util.Locale

private val dateTimeFormatter = DateTimeFormatter.ofPattern("yyyy-MM-dd HH:mm", Locale.ROOT)
    .withZone(ZoneId.of("Asia/Shanghai"))

internal fun displayDate(date: Instant): String = dateTimeFormatter.format(date)

internal fun chooseCampus(console: Console): Campus? =
    console.select("选择校区", Campus.entries) { it.displayName() }
