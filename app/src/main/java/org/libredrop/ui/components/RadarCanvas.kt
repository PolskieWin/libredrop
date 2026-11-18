package org.libredrop.ui.components

import androidx.compose.animation.core.*
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.Stroke
import org.libredrop.ui.theme.CyanPrimary
import kotlin.math.cos
import kotlin.math.sin

@Composable
fun RadarCanvas(
    isScanning: Boolean,
    modifier: Modifier = Modifier
) {
    val infiniteTransition = rememberInfiniteTransition(label = "radar")

    val pulseRadius by infiniteTransition.animateFloat(
        initialValue = 0.2f,
        targetValue = 1f,
        animationSpec = infiniteRepeatable(
            animation = tween(2400, easing = LinearEasing),
            repeatMode = RepeatMode.Restart
        ),
        label = "pulse"
    )

    val sweepAngle by infiniteTransition.animateFloat(
        initialValue = 0f,
        targetValue = 360f,
        animationSpec = infiniteRepeatable(
            animation = tween(3600, easing = LinearEasing),
            repeatMode = RepeatMode.Restart
        ),
        label = "sweep"
    )

    Canvas(modifier = modifier.fillMaxSize()) {
        val center = Offset(size.width / 2f, size.height / 2f)
        val maxRadius = minOf(size.width, size.height) / 2.2f

        // Draw concentric circles
        for (i in 1..4) {
            val r = maxRadius * (i / 4f)
            drawCircle(
                color = CyanPrimary.copy(alpha = 0.15f),
                radius = r,
                center = center,
                style = Stroke(width = 1.5f)
            )
        }

        // Draw pulsing circle
        if (isScanning) {
            drawCircle(
                color = CyanPrimary.copy(alpha = (1f - pulseRadius) * 0.4f),
                radius = maxRadius * pulseRadius,
                center = center,
                style = Stroke(width = 2.5f)
            )

            // Draw sweep line
            val rad = Math.toRadians(sweepAngle.toDouble())
            val lineEnd = Offset(
                x = (center.x + maxRadius * cos(rad)).toFloat(),
                y = (center.y + maxRadius * sin(rad)).toFloat()
            )
            drawLine(
                brush = Brush.linearGradient(
                    colors = listOf(CyanPrimary.copy(alpha = 0.8f), Color.Transparent),
                    start = center,
                    end = lineEnd
                ),
                start = center,
                end = lineEnd,
                strokeWidth = 3f
            )
        }

        // Center beacon dot
        drawCircle(
            color = CyanPrimary,
            radius = 6f,
            center = center
        )
    }
}
