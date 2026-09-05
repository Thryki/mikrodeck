# Reinicia o serviço da Native Instruments para ele reconhecer o Maschine Mikro MK3.
# Precisa de administrador.
#
# Motivo: o NIHardwareService subiu enquanto a interface do aparelho ainda estava com o
# driver WinUSB. Depois de devolver o driver HID, o serviço continua sem enxergar o aparelho
# até ser reiniciado.

$saida = "$env:USERPROFILE\AppData\Local\Temp\mikrodeck-captura"
New-Item -ItemType Directory -Force -Path $saida | Out-Null
Start-Transcript -Path (Join-Path $saida 'ni.txt') -Force | Out-Null

Write-Host "Fechando o Controller Editor, se estiver aberto..."
Get-Process 'Controller Editor' -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Seconds 2

Write-Host "Reiniciando o NIHardwareService..."
try {
    Restart-Service NIHardwareService -Force -ErrorAction Stop
    Write-Host "Servico reiniciado."
} catch {
    Write-Host "Falhou: $_"
}
Start-Sleep -Seconds 3
Get-Service NIHardwareService | Select-Object Name, Status | Format-Table -AutoSize

Write-Host "`nReiniciando a interface do aparelho para forcar nova deteccao..."
$hid = Get-PnpDevice | Where-Object { $_.InstanceId -match 'VID_17CC&PID_1700&MI_00' -and $_.Class -eq 'HIDClass' -and $_.InstanceId -match '^USB' }
foreach ($d in $hid) {
    Write-Host "  $($d.InstanceId)"
    try {
        Disable-PnpDevice -InstanceId $d.InstanceId -Confirm:$false -ErrorAction Stop
        Start-Sleep -Seconds 2
        Enable-PnpDevice -InstanceId $d.InstanceId -Confirm:$false -ErrorAction Stop
        Write-Host "  reiniciado"
    } catch { Write-Host "  falhou: $_" }
}
Start-Sleep -Seconds 3

Write-Host "`n=== Estado final ==="
Get-PnpDevice | Where-Object { $_.InstanceId -match 'VID_17CC' } | Select-Object Status, Class, FriendlyName | Format-Table -AutoSize
Write-Host "Agora abra o Controller Editor de novo e clique em Connect." -ForegroundColor Yellow
Stop-Transcript | Out-Null
Start-Sleep -Seconds 4
