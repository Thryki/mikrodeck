# Testa se a sequência de inicialização que copiamos do serviço da Native Instruments
# é suficiente para o aparelho passar a obedecer os comandos de LED.
#
# Precisa de administrador.
#
# Roteiro:
#  1. Para o serviço da NI, para ele não inicializar o aparelho por nós.
#  2. Reinicia o aparelho pelo Windows, para ele voltar ao estado "surdo".
#  3. Tenta acender os LEDs SEM a sequência. Deve falhar.
#  4. Roda a nossa sequência de inicialização e tenta de novo. É o que queremos que funcione.

$ErrorActionPreference = 'Continue'
$saida = '$env:USERPROFILE\AppData\Local\Temp\mikrodeck-captura'
New-Item -ItemType Directory -Force -Path $saida | Out-Null
Start-Transcript -Path (Join-Path $saida 'teste-init.txt') -Force | Out-Null

$envia = '$env:USERPROFILE\Documents\MikroDeck\mikrodeck\spike-hid\target\debug\envia.exe'
$marca = Join-Path $saida 'etapa.txt'

Write-Host "1) Parando o servico da NI"
Stop-Service NIHardwareService -Force -ErrorAction SilentlyContinue
Get-Process 'NIHostIntegrationAgent','NIHardwareAccessibilityHelper','nimc3cpl','Controller Editor' -ErrorAction SilentlyContinue |
    Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 3
Get-Service NIHardwareService | Select-Object Name, Status | Format-Table -AutoSize

Write-Host "2) Reiniciando o aparelho para ele voltar ao estado inicial"
$composto = (Get-PnpDevice | Where-Object { $_.InstanceId -match '^USB\\VID_17CC&PID_1700\\' }).InstanceId
Disable-PnpDevice -InstanceId $composto -Confirm:$false -ErrorAction SilentlyContinue
Start-Sleep -Seconds 4
Enable-PnpDevice -InstanceId $composto -Confirm:$false -ErrorAction SilentlyContinue
Start-Sleep -Seconds 6

Write-Host "3) Rodando SO a sequencia de inicializacao, segurando o aparelho aberto 20s"
Set-Content $marca 'com-init'
& $envia --init 11 20
Start-Sleep -Seconds 2

Write-Host "4) Depois que o programa fechou, os LEDs continuam acesos?"
Set-Content $marca 'depois'
Start-Sleep -Seconds 8

Set-Content $marca 'fim'
Write-Host "5) Religando o servico da NI"
Start-Service NIHardwareService -ErrorAction SilentlyContinue
Write-Host "Pronto." -ForegroundColor Green
Stop-Transcript | Out-Null
Start-Sleep -Seconds 3
