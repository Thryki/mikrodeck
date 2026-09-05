# Captura o tráfego USB do Maschine Mikro MK3 sem tempo limite.
# Grava até aparecer o arquivo de parada, criado de fora pelo Claude.
#
# Precisa de administrador.
#
# Por que só um controlador: a webcam fica no outro root hub e gera mais de 1 GB por minuto,
# o que travou o USBPcap nas tentativas anteriores. O Mikro sempre apareceu no USBPcap1.
#
# -A é obrigatório: sem ele o USBPcapCMD fica esperando escolha interativa e não grava nada.

$ErrorActionPreference = 'Continue'
$usbpcap = 'C:\Program Files\USBPcap\USBPcapCMD.exe'
$saida = "$env:USERPROFILE\AppData\Local\Temp\mikrodeck-captura"
New-Item -ItemType Directory -Force -Path $saida | Out-Null
Start-Transcript -Path (Join-Path $saida 'livre.txt') -Force | Out-Null

$hub = '\\.\USBPcap1'
$arq = Join-Path $saida 'livre.pcap'
$parar = Join-Path $saida 'PARAR.txt'
$pronto = Join-Path $saida 'GRAVANDO.txt'
Remove-Item $arq, $parar, $pronto -ErrorAction SilentlyContinue

Write-Host "Capturando em $hub -> $arq" -ForegroundColor Cyan
$p = Start-Process -FilePath $usbpcap -ArgumentList @('-d', $hub, '-o', $arq, '-A') -PassThru -WindowStyle Hidden
Start-Sleep -Seconds 2
Set-Content $pronto 'ok'

Write-Host ""
Write-Host "  ==================================================" -ForegroundColor Yellow
Write-Host "   GRAVANDO. Sem pressa, sem contagem." -ForegroundColor Yellow
Write-Host "   1) Desconecte o cabo, espere 3s, reconecte." -ForegroundColor Yellow
Write-Host "   2) Abra o Maschine 2 COMO ADMINISTRADOR." -ForegroundColor Yellow
Write-Host "   3) Quando ele conectar e os LEDs acenderem," -ForegroundColor Yellow
Write-Host "      avise o Claude no chat. Ele para a gravacao." -ForegroundColor Yellow
Write-Host "  ==================================================" -ForegroundColor Yellow
Write-Host ""

$t0 = Get-Date
while (-not (Test-Path $parar)) {
    Start-Sleep -Milliseconds 500
    if (((Get-Date) - $t0).TotalMinutes -gt 15) {
        Write-Host "Limite de seguranca de 15 minutos atingido." -ForegroundColor Red
        break
    }
}

Write-Host "`nParando a captura..." -ForegroundColor Green
try { Stop-Process -Id $p.Id -Force -ErrorAction Stop } catch {}
Start-Sleep -Seconds 2

if (Test-Path $arq) {
    Write-Host ("Arquivo: {0}  {1:N0} bytes" -f $arq, (Get-Item $arq).Length)
}
Write-Host "Pronto." -ForegroundColor Green
Stop-Transcript | Out-Null
Start-Sleep -Seconds 3
