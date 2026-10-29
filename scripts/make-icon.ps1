Add-Type -AssemblyName System.Drawing

$size = 512
$bmp = New-Object System.Drawing.Bitmap($size, $size)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
$g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
$g.Clear([System.Drawing.Color]::Transparent)

function New-RoundedPath([float]$x, [float]$y, [float]$w, [float]$h, [float]$r) {
  $p = New-Object System.Drawing.Drawing2D.GraphicsPath
  $d = $r * 2
  $p.AddArc($x, $y, $d, $d, 180, 90)
  $p.AddArc($x + $w - $d, $y, $d, $d, 270, 90)
  $p.AddArc($x + $w - $d, $y + $h - $d, $d, $d, 0, 90)
  $p.AddArc($x, $y + $h - $d, $d, $d, 90, 90)
  $p.CloseFigure()
  return $p
}

# Fundo arredondado com gradiente
$bgPath = New-RoundedPath 16 16 480 480 112
$bgBrush = New-Object System.Drawing.Drawing2D.LinearGradientBrush(
  (New-Object System.Drawing.Point(40, 30)),
  (New-Object System.Drawing.Point(470, 490)),
  [System.Drawing.Color]::FromArgb(255, 16, 23, 33),
  [System.Drawing.Color]::FromArgb(255, 7, 10, 15)
)
$g.FillPath($bgBrush, $bgPath)

# Borda sutil
$borderPen = New-Object System.Drawing.Pen([System.Drawing.Color]::FromArgb(60, 120, 200, 230), 2)
$g.DrawPath($borderPen, $bgPath)

# Brilho ciano no topo
$glowPath = New-RoundedPath 16 16 480 300 112
$glowBrush = New-Object System.Drawing.Drawing2D.LinearGradientBrush(
  (New-Object System.Drawing.Point(0, 16)),
  (New-Object System.Drawing.Point(0, 300)),
  [System.Drawing.Color]::FromArgb(70, 34, 211, 238),
  [System.Drawing.Color]::FromArgb(0, 34, 211, 238)
)
$g.FillPath($glowBrush, $glowPath)

# Setas de download (três linhas)
$arrowBrush = New-Object System.Drawing.Drawing2D.LinearGradientBrush(
  (New-Object System.Drawing.Point(180, 150)),
  (New-Object System.Drawing.Point(340, 350)),
  [System.Drawing.Color]::FromArgb(255, 94, 234, 212),
  [System.Drawing.Color]::FromArgb(255, 34, 211, 238)
)

# Haste da seta
$stemPath = New-RoundedPath 232 128 48 150 24
$g.FillPath($arrowBrush, $stemPath)

# Ponta da seta
$head = New-Object System.Drawing.Drawing2D.GraphicsPath
$head.AddPolygon(@(
  (New-Object System.Drawing.Point(160, 246)),
  (New-Object System.Drawing.Point(352, 246)),
  (New-Object System.Drawing.Point(256, 372))
))
$g.FillPath($arrowBrush, $head)

# Bandeja / base
$trayPen = New-Object System.Drawing.Pen([System.Drawing.Color]::FromArgb(255, 226, 240, 250), 26)
$trayPen.StartCap = [System.Drawing.Drawing2D.LineCap]::Round
$trayPen.EndCap = [System.Drawing.Drawing2D.LineCap]::Round
$g.DrawLine($trayPen, 150, 402, 362, 402)

# Glow externo suave
$glowPen = New-Object System.Drawing.Pen([System.Drawing.Color]::FromArgb(45, 34, 211, 238), 6)
$g.DrawPath($glowPen, $bgPath)

$g.Dispose()
$bmp.Save("C:\Users\spekv\vidmanager\icon-source.png", [System.Drawing.Imaging.ImageFormat]::Png)
$bmp.Dispose()

Write-Output "icon-source.png gerado"
