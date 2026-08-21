Add-Type -AssemblyName System.Drawing
New-Item -ItemType Directory -Force -Path "d:\src\copyfile\assets" | Out-Null

$sizes = @(16, 32, 48, 64, 128, 256)
$ms = New-Object System.IO.MemoryStream
$bw = New-Object System.IO.BinaryWriter($ms)

# ICONDIR
$bw.Write([uint16]0) # Reserved
$bw.Write([uint16]1) # Type 1 = ICO
$bw.Write([uint16]$sizes.Count) # Count

$pngStreams = @()
foreach ($sz in $sizes) {
    $bmp = New-Object System.Drawing.Bitmap($sz, $sz)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $g.Clear([System.Drawing.Color]::Transparent)
    
    # Rounded background
    $rect = New-Object System.Drawing.Rectangle(1, 1, ($sz - 2), ($sz - 2))
    $brush = New-Object System.Drawing.Drawing2D.LinearGradientBrush($rect, [System.Drawing.Color]::FromArgb(37, 99, 235), [System.Drawing.Color]::FromArgb(16, 185, 129), 45.0)
    
    $g.FillEllipse($brush, 1, 1, ($sz - 3), ($sz - 3))
    
    $pen = New-Object System.Drawing.Pen([System.Drawing.Color]::FromArgb(255, 255, 255), [Math]::Max(1.0, $sz / 24.0))
    $g.DrawEllipse($pen, 1, 1, ($sz - 3), ($sz - 3))
    
    # Draw lightning symbol in white
    $whiteBrush = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::White)
    $p1 = New-Object System.Drawing.PointF(($sz * 0.52), ($sz * 0.18))
    $p2 = New-Object System.Drawing.PointF(($sz * 0.28), ($sz * 0.52))
    $p3 = New-Object System.Drawing.PointF(($sz * 0.48), ($sz * 0.52))
    $p4 = New-Object System.Drawing.PointF(($sz * 0.44), ($sz * 0.82))
    $p5 = New-Object System.Drawing.PointF(($sz * 0.72), ($sz * 0.46))
    $p6 = New-Object System.Drawing.PointF(($sz * 0.52), ($sz * 0.46))
    $points = [System.Drawing.PointF[]]@($p1, $p2, $p3, $p4, $p5, $p6)
    
    $g.FillPolygon($whiteBrush, $points)
    $g.Dispose()
    
    $pngMs = New-Object System.IO.MemoryStream
    $bmp.Save($pngMs, [System.Drawing.Imaging.ImageFormat]::Png)
    $pngStreams += ,@($sz, $pngMs.ToArray())
    $bmp.Dispose()
}

$offset = 6 + (16 * $sizes.Count)
foreach ($item in $pngStreams) {
    $sz = $item[0]
    $bytes = $item[1]
    $w = if ($sz -ge 256) { 0 } else { [byte]$sz }
    $h = if ($sz -ge 256) { 0 } else { [byte]$sz }
    
    $bw.Write([byte]$w)
    $bw.Write([byte]$h)
    $bw.Write([byte]0) # Color count
    $bw.Write([byte]0) # Reserved
    $bw.Write([uint16]1) # Color planes
    $bw.Write([uint16]32) # Bits per pixel
    $bw.Write([uint32]$bytes.Length) # Size in bytes
    $bw.Write([uint32]$offset) # Offset
    $offset += $bytes.Length
}

foreach ($item in $pngStreams) {
    $bytes = $item[1]
    $bw.Write($bytes)
}

[System.IO.File]::WriteAllBytes("d:\src\copyfile\assets\copyfile.ico", $ms.ToArray())
Write-Output "assets\copyfile.ico generated successfully"
