# Diagnóstico do USBPcap: descobre por que nenhum root hub aparece.
# Roda como administrador.
$saida = '$env:USERPROFILE\AppData\Local\Temp\mikrodeck-captura'
New-Item -ItemType Directory -Force -Path $saida | Out-Null
Start-Transcript -Path (Join-Path $saida 'diag.txt') -Force | Out-Null

$id = [Security.Principal.WindowsIdentity]::GetCurrent()
$pr = New-Object Security.Principal.WindowsPrincipal($id)
Write-Host "Administrador: $($pr.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator))"

Write-Host "`n=== Serviço USBPcap ==="
Get-Service USBPcap -ErrorAction SilentlyContinue | Format-List Name, Status, StartType
try {
    Start-Service USBPcap -ErrorAction Stop
    Write-Host "Start-Service USBPcap: OK"
} catch {
    Write-Host "Start-Service USBPcap falhou: $_"
}
Get-Service USBPcap -ErrorAction SilentlyContinue | Format-List Name, Status

Write-Host "`n=== UpperFilters dos controladores USB (é onde o USBPcap se enfia) ==="
$classe = 'HKLM:\SYSTEM\CurrentControlSet\Control\Class\{36fc9e60-c465-11cf-8056-444553540000}'
if (Test-Path $classe) {
    $v = Get-ItemProperty -Path $classe -Name UpperFilters -ErrorAction SilentlyContinue
    Write-Host "UpperFilters da classe USB: $($v.UpperFilters -join ', ')"
} else {
    Write-Host "Chave de classe USB não encontrada"
}

Write-Host "`n=== Root hubs no sistema ==="
Get-PnpDevice -Class USB -ErrorAction SilentlyContinue |
    Where-Object { $_.FriendlyName -match 'Root Hub|Concentrador Raiz|Hub Raiz' } |
    Select-Object Status, FriendlyName, InstanceId | Format-Table -AutoSize -Wrap

Write-Host "`n=== USBPcapCMD --extcap-interfaces (saída bruta) ==="
$r = & 'C:\Program Files\USBPcap\USBPcapCMD.exe' --extcap-interfaces 2>&1 | Out-String
Write-Host "[$r]"

Write-Host "`n=== Existe \\.\USBPcap1 ? ==="
foreach ($n in 1..4) {
    $existe = Test-Path "\\.\USBPcap$n"
    Write-Host "USBPcap${n}: $existe"
}

Stop-Transcript | Out-Null
Start-Sleep -Seconds 3
