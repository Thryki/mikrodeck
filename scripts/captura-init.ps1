# Captura a sequência de inicialização que o serviço da Native Instruments manda para o
# Maschine Mikro MK3. É a única coisa que falta para o motor do MikroDeck ficar independente.
#
# Precisa de administrador.
#
# Em vez de pedir para alguém desconectar o cabo, o script desabilita e reabilita o
# dispositivo pelo Windows, o que força a mesma reenumeração e faz o serviço da NI
# reinicializar o aparelho. Tudo automático.
#
# -A faz o USBPcap capturar todos os dispositivos do controlador. Sem isso ele fica
# esperando uma escolha interativa e não grava nada.

$ErrorActionPreference = 'Continue'
$usbpcap = 'C:\Program Files\USBPcap\USBPcapCMD.exe'
$saida = '$env:USERPROFILE\AppData\Local\Temp\mikrodeck-captura'
New-Item -ItemType Directory -Force -Path $saida | Out-Null
Start-Transcript -Path (Join-Path $saida 'init.txt') -Force | Out-Null

# O nó a reiniciar é o dispositivo composto, para a reenumeração ser completa.
$composto = (Get-PnpDevice | Where-Object {
    $_.InstanceId -match '^USB\\VID_17CC&PID_1700\\' }).InstanceId
if (-not $composto) { Write-Host "Mikro nao encontrado"; Stop-Transcript; Start-Sleep 10; exit 1 }
Write-Host "Dispositivo: $composto" -ForegroundColor Cyan

$listagem = & $usbpcap --extcap-interfaces 2>&1 | Out-String
$hubs = [regex]::Matches($listagem, '\\\\\.\\USBPcap\d+') | ForEach-Object { $_.Value } | Select-Object -Unique
Write-Host "Capturando em: $($hubs -join ', ')" -ForegroundColor Cyan

$procs = @()
foreach ($h in $hubs) {
    $n = ($h -replace '[\\\.]', '')
    $arq = Join-Path $saida "init-$n.pcap"
    Remove-Item $arq -ErrorAction SilentlyContinue
    $p = Start-Process -FilePath $usbpcap -ArgumentList @('-d', $h, '-o', $arq, '-A') -PassThru -WindowStyle Hidden
    $procs += [pscustomobject]@{ Proc = $p; Arquivo = $arq }
}
Start-Sleep -Seconds 3

Write-Host ""
Write-Host "  ==================================================" -ForegroundColor Yellow
Write-Host "   1) DESCONECTE O CABO USB DO MIKRO AGORA." -ForegroundColor Yellow
Write-Host "      Espere 3 segundos e CONECTE de novo." -ForegroundColor Yellow
Write-Host "      (desligar pelo Windows nao vale, precisa" -ForegroundColor Yellow
Write-Host "       tirar o cabo mesmo para cortar a energia)" -ForegroundColor Yellow
Write-Host "   2) Assim que reconectar, ABRA O MASCHINE 2" -ForegroundColor Yellow
Write-Host "      e espere ele conectar com o aparelho." -ForegroundColor Yellow
Write-Host "  ==================================================" -ForegroundColor Yellow
Write-Host ""
foreach ($s in 90..1) {
    Write-Host "`r  gravando... $s   " -NoNewline -ForegroundColor Green
    Start-Sleep -Seconds 1
}
Write-Host ""

Write-Host "`nParando a captura..."
foreach ($x in $procs) { try { Stop-Process -Id $x.Proc.Id -Force -ErrorAction Stop } catch {} }
Start-Sleep -Seconds 2

Write-Host "`n=== Arquivos ==="
foreach ($x in $procs) {
    if (Test-Path $x.Arquivo) {
        Write-Host ("{0}  {1:N0} bytes" -f (Split-Path $x.Arquivo -Leaf), (Get-Item $x.Arquivo).Length)
    }
}
Write-Host "`n=== Estado do aparelho ==="
Get-PnpDevice | Where-Object { $_.InstanceId -match 'VID_17CC' } |
    Select-Object Status, Class, FriendlyName | Format-Table -AutoSize

Write-Host "Pronto." -ForegroundColor Green
Stop-Transcript | Out-Null
Start-Sleep -Seconds 4
