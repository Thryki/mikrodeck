# Testa se o controle de LED continua funcionando com o serviço da Native Instruments PARADO.
# É o teste que define se o MikroDeck pode ser independente do software da NI.
# Precisa de administrador. No fim, religa o serviço.

$saida = "$env:USERPROFILE\AppData\Local\Temp\mikrodeck-captura"
New-Item -ItemType Directory -Force -Path $saida | Out-Null
Start-Transcript -Path (Join-Path $saida 'sem-ni.txt') -Force | Out-Null

$envia = "$env:USERPROFILE\Documents\MikroDeck\mikrodeck\spike-hid\target\debug\envia.exe"
$vermelho = @('--preenche', '0x80', '0x07', '--tam', '81')   # cor 1, intensidade 3
$branco  = @('--preenche', '0x80', '0x47', '--tam', '81')    # cor 17, intensidade 3

Write-Host "=== Parando o NIHardwareService ==="
Stop-Service NIHardwareService -Force -ErrorAction SilentlyContinue
Get-Process 'NIHostIntegrationAgent','NIHardwareAccessibilityHelper','nimc3cpl' -ErrorAction SilentlyContinue |
    Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 3
Get-Service NIHardwareService | Select-Object Name, Status | Format-Table -AutoSize
Get-Process | Where-Object { $_.ProcessName -match 'NIH|nimc3|Controller Editor' } |
    Select-Object ProcessName | Format-Table -AutoSize

Write-Host "`n=== Escrevendo LEDs VERMELHOS sem o servico da NI ==="
& $envia @vermelho
Write-Host "AGUARDE 10 segundos (a foto sera tirada aqui)"
Start-Sleep -Seconds 10

Write-Host "`n=== Escrevendo LEDs BRANCOS ==="
& $envia @branco
Start-Sleep -Seconds 8

Write-Host "`n=== Religando o NIHardwareService ==="
Start-Service NIHardwareService -ErrorAction SilentlyContinue
Start-Sleep -Seconds 2
Get-Service NIHardwareService | Select-Object Name, Status | Format-Table -AutoSize

Stop-Transcript | Out-Null
Start-Sleep -Seconds 3
